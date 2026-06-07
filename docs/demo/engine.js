/**
 * greenbook evaluation engine - JavaScript port of src/evaluate.rs.
 *
 * The Rust crate remains the source of truth; this is a faithful client-side
 * port so the static demo can evaluate live (change a patient's age or doses and
 * recompute, with no server). It is validated against the Rust CLI's JSON output
 * for the bundled fixtures - see docs/demo/README.md.
 *
 * Two questions, per docs/adr/0001:
 *   - Conformance: did the patient get the doses the Green Book named, at valid
 *     ages? Doses match a series by PRODUCT CLASS. Drives the headline status.
 *   - Coverage: which diseases are they protected against? Aggregated over the
 *     ANTIGENS of every product received. (Deferred in Rust; computed here for
 *     the demo's "layers" view, clearly labelled as the separate question.)
 *
 * Everything is exposed on `window.Greenbook`.
 */
(function () {
  'use strict';

  const DAY_MS = 86400000;

  // --- Dates (all UTC midnight, so day arithmetic is exact) ----------------

  function parseDate(s) {
    return new Date(s + 'T00:00:00Z');
  }
  function fmtDate(d) {
    return d.toISOString().slice(0, 10);
  }
  // Add calendar months, clamping the day to the last valid day of the target
  // month (matches chrono's checked_add_months: Jan 31 + 1mo -> Feb 28/29).
  function addMonths(date, n) {
    const y = date.getUTCFullYear();
    const target = date.getUTCMonth() + n;
    const ty = y + Math.floor(target / 12);
    const tm = ((target % 12) + 12) % 12;
    const lastDay = new Date(Date.UTC(ty, tm + 1, 0)).getUTCDate();
    const td = Math.min(date.getUTCDate(), lastDay);
    return new Date(Date.UTC(ty, tm, td));
  }
  function addDays(date, n) {
    return new Date(date.getTime() + n * DAY_MS);
  }

  // --- AgeOffset ("8 weeks", "3 years 4 months", "14 weeks 6 days") --------

  function parseAgeOffset(s) {
    const tokens = String(s).trim().split(/\s+/);
    const off = { years: 0, months: 0, weeks: 0, days: 0 };
    for (let i = 0; i < tokens.length; i += 2) {
      const n = parseInt(tokens[i], 10);
      const unit = (tokens[i + 1] || '').toLowerCase();
      if (unit.startsWith('year')) off.years = n;
      else if (unit.startsWith('month')) off.months = n;
      else if (unit.startsWith('week')) off.weeks = n;
      else if (unit.startsWith('day')) off.days = n;
      else throw new Error('bad age offset: ' + s);
    }
    return off;
  }
  // The date an offset from DOB falls on: add months (clamped) then the days.
  function ageOffsetToDate(offsetStr, dob) {
    const off = parseAgeOffset(offsetStr);
    const withMonths = addMonths(dob, off.years * 12 + off.months);
    return addDays(withMonths, off.weeks * 7 + off.days);
  }

  // Compact human age, e.g. "3mo 2w 6d" - matches age_between() in evaluate.rs.
  function ageBetween(dob, when) {
    let months = 0;
    while (addMonths(dob, months + 1).getTime() <= when.getTime()) months++;
    const years = Math.floor(months / 12);
    const remMonths = months % 12;
    const monthAnchor = addMonths(dob, months);
    const daysAfter = Math.max(0, Math.round((when.getTime() - monthAnchor.getTime()) / DAY_MS));
    const weeks = Math.floor(daysAfter / 7);
    const leftover = daysAfter % 7;
    const parts = [];
    if (years > 0) parts.push(years + 'y');
    if (remMonths > 0) parts.push(remMonths + 'mo');
    if (weeks > 0) parts.push(weeks + 'w');
    if (leftover > 0 || parts.length === 0) parts.push(leftover + 'd');
    return parts.join(' ');
  }

  // --- Product map ----------------------------------------------------------

  // Index the product-map file (products.product[]) by SNOMED code.
  function makeProductIndex(productsFile) {
    const byCode = new Map();
    for (const p of productsFile.product || []) byCode.set(p.code, p);
    return byCode;
  }
  const classFor = (idx, code) => (idx.has(code) ? idx.get(code).product_class : null);
  const antigensFor = (idx, code) => (idx.has(code) ? idx.get(code).antigens : null);

  // --- Eligibility (spec/standard.md "Eligibility check") -------------------

  function checkEligibility(eligibility, record) {
    const gender = record.gender;
    const isMale = gender === 'male';
    const isFemale = gender === 'female';
    const sexUnknown = !isMale && !isFemale; // other / unknown / absent

    const maleCohort = () => {
      const cutoff = eligibility.male_born_on_or_after;
      if (cutoff && parseDate(record.dob).getTime() < parseDate(cutoff).getTime()) {
        return { eligible: false, uncertain: false, note: `male born before ${cutoff} - outside the eligible birth cohort` };
      }
      return { eligible: true, uncertain: false, note: null };
    };

    switch (eligibility.population) {
      case 'female':
        if (isFemale) return { eligible: true, uncertain: false, note: null };
        if (sexUnknown) return { eligible: true, uncertain: true, note: 'female-only series; patient gender is other/unknown, treated as eligible' };
        return { eligible: false, uncertain: false, note: null };
      case 'male':
        if (isMale) return maleCohort();
        if (sexUnknown) return { eligible: true, uncertain: true, note: 'male-only series; patient gender is other/unknown, treated as eligible' };
        return { eligible: false, uncertain: false, note: null };
      default: // "all", possibly with a male birth-cohort restriction
        if (isMale) return maleCohort();
        if (sexUnknown && eligibility.male_born_on_or_after)
          return { eligible: true, uncertain: true, note: 'series restricts males by birth cohort; patient gender is other/unknown, treated as eligible' };
        return { eligible: true, uncertain: false, note: null };
    }
  }

  // --- Per-series evaluation ------------------------------------------------

  function evaluateSeries(series, record, idx, evaluatedAt) {
    const dob = parseDate(record.dob);
    const elig = checkEligibility(series.eligibility, record);
    const notes = elig.note ? [elig.note] : [];
    const dosesExpected = series.dose.length;

    // How many defined doses are *due* by the evaluation date?
    const dosesDue = series.dose.filter((d) => {
      const dueAt = ageOffsetToDate(d.earliest_age || d.target_age, dob);
      return dueAt.getTime() <= evaluatedAt.getTime();
    }).length;

    if (!elig.eligible) {
      return {
        series_id: series.id, display_name: series.display_name,
        status: 'not_applicable', eligible: false, eligibility_uncertain: elig.uncertain,
        doses_expected: dosesExpected, doses_due: dosesDue, doses_valid: 0,
        up_to_date_for_age: true, doses_recorded: [], notes,
      };
    }

    // Conformance matching: by product class, not antigen overlap (ADR 0001).
    const matched = record.immunisations
      .filter((imm) => classFor(idx, imm.vaccine_code) === series.product_class)
      .slice()
      .sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : 0));

    const recorded = [];
    let lastValidDate = null;
    let nextDoseIdx = 0;

    for (const imm of matched) {
      const scheduleNotes = [];
      let withinSchedule = true;
      let assigned = null;
      const immDate = parseDate(imm.date);

      if (nextDoseIdx >= series.dose.length) {
        withinSchedule = false;
        scheduleNotes.push('extra dose beyond the expected count for this series');
      } else {
        const dose = series.dose[nextDoseIdx];
        assigned = dose.number;
        if (dose.earliest_age) {
          const earliest = ageOffsetToDate(dose.earliest_age, dob);
          if (immDate.getTime() < earliest.getTime()) {
            withinSchedule = false;
            scheduleNotes.push(`given before earliest_age ${dose.earliest_age} (${fmtDate(earliest)}) - outside standard schedule`);
          }
        }
        if (dose.latest_age) {
          const latest = ageOffsetToDate(dose.latest_age, dob);
          if (immDate.getTime() > latest.getTime()) {
            withinSchedule = false;
            scheduleNotes.push(`given after latest_age ${dose.latest_age} (${fmtDate(latest)}) - outside standard schedule`);
          }
        }
        if (dose.min_interval_from_previous && lastValidDate) {
          const earliestByInterval = ageOffsetToDate(dose.min_interval_from_previous, lastValidDate);
          if (immDate.getTime() < earliestByInterval.getTime()) {
            withinSchedule = false;
            scheduleNotes.push(`interval from previous dose < ${dose.min_interval_from_previous} (needs to be on/after ${fmtDate(earliestByInterval)}) - outside standard schedule`);
          }
        }
      }

      recorded.push({
        date: imm.date, age_at_dose: ageBetween(dob, immDate),
        vaccine_code: imm.vaccine_code, display: imm.display,
        assigned_dose_number: assigned, within_schedule: withinSchedule, schedule_notes: scheduleNotes,
      });

      if (withinSchedule) { lastValidDate = immDate; nextDoseIdx++; }
    }

    const dosesValid = recorded.filter((d) => d.within_schedule).length;
    const status = dosesValid >= dosesExpected ? 'complete' : dosesValid > 0 ? 'partial' : 'none';

    return {
      series_id: series.id, display_name: series.display_name, status,
      eligible: true, eligibility_uncertain: elig.uncertain,
      doses_expected: dosesExpected, doses_due: dosesDue, doses_valid: dosesValid,
      up_to_date_for_age: dosesValid >= dosesDue, doses_recorded: recorded, notes,
    };
  }

  // Doses that match no series at all (unknown code, or a known product whose
  // class no series in this schedule uses).
  function findUnmatched(record, schedule, idx) {
    const scheduleClasses = new Set(schedule.series.map((s) => s.product_class));
    const out = [];
    for (const imm of record.immunisations) {
      const cls = classFor(idx, imm.vaccine_code);
      if (cls == null) {
        out.push({ date: imm.date, vaccine_code: imm.vaccine_code, display: imm.display, reason: 'unknown product code (not in the product map)' });
      } else if (!scheduleClasses.has(cls)) {
        out.push({ date: imm.date, vaccine_code: imm.vaccine_code, display: imm.display, reason: `product class "${cls}" has no series in this schedule version` });
      }
    }
    return out;
  }

  function aggregate(seriesStatuses) {
    const applicable = seriesStatuses.filter((s) => s.eligible);
    if (applicable.length === 0) return 'unknown';
    const totalValid = applicable.reduce((a, s) => a + s.doses_valid, 0);
    const totalDue = applicable.reduce((a, s) => a + s.doses_due, 0);
    const allUpToDate = applicable.every((s) => s.up_to_date_for_age);
    if (totalValid === 0) return totalDue === 0 ? 'up_to_date_for_age' : 'unvaccinated';
    return allUpToDate ? 'up_to_date_for_age' : 'behind_for_age';
  }

  // --- Coverage (the second question; demo-only) ----------------------------
  // Which diseases is the patient protected against, across every product
  // received? Independent of series/conformance.
  function antigenCoverage(record, schedule, idx) {
    return schedule.antigen.map((ant) => {
      const byDoses = record.immunisations.filter((imm) => {
        const ants = antigensFor(idx, imm.vaccine_code);
        return ants && ants.includes(ant.id);
      });
      return { id: ant.id, display_name: ant.display_name, covered: byDoses.length > 0, by_doses: byDoses };
    });
  }

  // --- Top level ------------------------------------------------------------

  function evaluate(record, schedule, productsFile, evaluatedAtStr) {
    const idx = makeProductIndex(productsFile);
    const evaluatedAt = parseDate(evaluatedAtStr);
    const bySeries = schedule.series.map((s) => evaluateSeries(s, record, idx, evaluatedAt));
    const status = aggregate(bySeries);
    const eligible = bySeries.filter((s) => s.eligible);
    const fullyVaccinated = eligible.length > 0 && eligible.every((s) => s.status === 'complete');

    return {
      status,
      fully_vaccinated: fullyVaccinated,
      evaluated_at: evaluatedAtStr,
      schedule_version: schedule.schedule.valid_from,
      by_series: bySeries,
      unmatched_doses: findUnmatched(record, schedule, idx),
      by_antigen: antigenCoverage(record, schedule, idx),
    };
  }

  window.Greenbook = {
    evaluate, makeProductIndex, classFor, antigensFor,
    ageOffsetToDate, parseAgeOffset, ageBetween, parseDate, fmtDate, addMonths, addDays,
  };
})();
