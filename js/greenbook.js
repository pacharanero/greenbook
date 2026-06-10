/**
 * greenbook evaluation engine - the JavaScript implementation.
 *
 * Rust is the reference implementation (rust/); this is an independent JS one,
 * kept in step via the shared conformance suite (conformance/, run from
 * js/test/conformance.mjs). It parses a FHIR bundle and evaluates a patient
 * against a schedule entirely client-side - which is what lets the demo run
 * with no server.
 *
 * Two questions, per spec/conformance-vs-coverage.md:
 *   - Conformance: did the patient get the doses the Green Book named, at valid
 *     ages? Doses match a series by PRODUCT CLASS. Drives the headline status.
 *   - Coverage: which diseases are they protected against? Aggregated over the
 *     ANTIGENS of every product received. (Not part of the reference engine /
 *     conformance goldens; computed here for the demo's "layers" view.)
 *
 * A product class can serve several series (e.g. MMR -> first + second dose). It
 * is evaluated as one programme: doses are allocated across the class's series
 * slots by date, with the recorded dose number / procedure code as cross-checks.
 * Duplicate "echoes" (same procedure code) are dropped before matching.
 *
 * Usable in both the browser (as the global `Greenbook`) and Node (as a module
 * via `import`/`require`), so the same file backs the demo and the tests.
 */
(function (root, factory) {
  const api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api; // Node / CommonJS
  } else {
    root.Greenbook = api; // browser global
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
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

  // --- Dose-sequence helpers ------------------------------------------------

  // Parse an explicit dose number from a procedure code's display text, e.g.
  // "Administration of second dose of ... (procedure)" -> 2. None if not stated
  // (the first dose is often recorded with the generic administration code).
  function doseFromProcedure(display) {
    if (!display) return null;
    const d = display.toLowerCase();
    const words = [['first', 1], ['second', 2], ['third', 3], ['fourth', 4], ['fifth', 5]];
    for (const [w, n] of words) if (d.includes(`${w} dose`)) return n;
    return null;
  }

  // Detect duplicate "echoes": records sharing a procedure code are the same act.
  // Keep the earliest, report the rest. (record.immunisations is date-sorted.)
  function detectDuplicates(immunisations) {
    const seen = new Map(); // procedure_code -> earliest date
    const kept = [];
    const duplicates = [];
    for (const imm of immunisations) {
      const code = imm.procedure_code || null;
      if (code != null) {
        if (seen.has(code)) {
          duplicates.push({ date: imm.date, vaccine_code: imm.vaccine_code, display: imm.display, procedure_code: code, duplicate_of: seen.get(code) });
        } else {
          seen.set(code, imm.date);
          kept.push(imm);
        }
      } else {
        kept.push(imm);
      }
    }
    return { kept, duplicates };
  }

  // How many of a series' doses are due by the evaluation date.
  function dosesDueFor(series, dob, evaluatedAt) {
    return series.dose.filter((d) => ageOffsetToDate(d.earliest_age || d.target_age, dob).getTime() <= evaluatedAt.getTime()).length;
  }

  // --- Per-class evaluation -------------------------------------------------
  // All series sharing one product class are evaluated as one programme: the
  // class's doses are allocated across the series' slots in date order.
  function evaluateClassGroup(group, classDoses, record, evaluatedAt) {
    const dob = parseDate(record.dob);
    const eligs = group.map((s) => checkEligibility(s.eligibility, record));

    // Slots from the eligible series, ordered by the date each dose targets.
    const slots = [];
    group.forEach((s, gi) => { if (eligs[gi].eligible) for (const dose of s.dose) slots.push({ gi, dose }); });
    slots.sort((a, b) => ageOffsetToDate(a.dose.target_age, dob).getTime() - ageOffsetToDate(b.dose.target_age, dob).getTime());

    let lastEligibleGi = -1;
    for (let gi = group.length - 1; gi >= 0; gi--) if (eligs[gi].eligible) { lastEligibleGi = gi; break; }

    const recordedByGi = group.map(() => []);
    let lastValidDate = null;

    classDoses.forEach((imm, i) => {
      const scheduleNotes = [];
      const flags = [];
      let withinSchedule = true;
      let assigned = null;
      const immDate = parseDate(imm.date);
      let targetGi;

      if (i < slots.length) {
        const slot = slots[i];
        assigned = slot.dose.number;
        if (slot.dose.earliest_age) {
          const earliest = ageOffsetToDate(slot.dose.earliest_age, dob);
          if (immDate.getTime() < earliest.getTime()) { withinSchedule = false; scheduleNotes.push(`given before earliest_age ${slot.dose.earliest_age} (${fmtDate(earliest)}) - outside standard schedule`); }
        }
        if (slot.dose.latest_age) {
          const latest = ageOffsetToDate(slot.dose.latest_age, dob);
          if (immDate.getTime() > latest.getTime()) { withinSchedule = false; scheduleNotes.push(`given after latest_age ${slot.dose.latest_age} (${fmtDate(latest)}) - outside standard schedule`); }
        }
        if (slot.dose.min_interval_from_previous && lastValidDate) {
          const earliestByInterval = ageOffsetToDate(slot.dose.min_interval_from_previous, lastValidDate);
          if (immDate.getTime() < earliestByInterval.getTime()) { withinSchedule = false; scheduleNotes.push(`interval from previous dose < ${slot.dose.min_interval_from_previous} (needs to be on/after ${fmtDate(earliestByInterval)}) - outside standard schedule`); }
        }
        // Cross-check the date-derived dose number; flag disagreement, never override.
        if (imm.dose_number != null && imm.dose_number !== slot.dose.number) {
          flags.push(`recorded dose number ${imm.dose_number} disagrees with position-by-date (dose ${slot.dose.number}) - sequence may be mis-keyed`);
        }
        const procN = doseFromProcedure(imm.procedure_display);
        if (procN != null && procN !== slot.dose.number) {
          flags.push(`procedure code indicates dose ${procN} but by date this is dose ${slot.dose.number}`);
        }
        targetGi = slot.gi;
      } else {
        withinSchedule = false;
        scheduleNotes.push('extra dose beyond the expected count for this class');
        if (lastEligibleGi < 0) return; // no eligible series in the group; ignore
        targetGi = lastEligibleGi;
      }

      recordedByGi[targetGi].push({
        date: imm.date,
        age_at_dose: ageBetween(dob, immDate),
        vaccine_code: imm.vaccine_code,
        display: imm.display,
        assigned_dose_number: assigned,
        within_schedule: withinSchedule,
        schedule_notes: scheduleNotes,
        flags,
      });
      if (withinSchedule) lastValidDate = immDate;
    });

    return group.map((s, gi) => {
      const elig = eligs[gi];
      const notes = elig.note ? [elig.note] : [];
      const dosesExpected = s.dose.length;
      const dosesDue = dosesDueFor(s, dob, evaluatedAt);
      if (!elig.eligible) {
        return {
          series_id: s.id, display_name: s.display_name, status: 'not_applicable',
          eligible: false, eligibility_uncertain: elig.uncertain,
          doses_expected: dosesExpected, doses_due: dosesDue, doses_valid: 0,
          up_to_date_for_age: true, doses_recorded: [], notes,
        };
      }
      const recorded = recordedByGi[gi];
      const dosesValid = recorded.filter((d) => d.within_schedule).length;
      const status = dosesValid >= dosesExpected ? 'complete' : dosesValid > 0 ? 'partial' : 'none';
      return {
        series_id: s.id, display_name: s.display_name, status,
        eligible: true, eligibility_uncertain: elig.uncertain,
        doses_expected: dosesExpected, doses_due: dosesDue, doses_valid: dosesValid,
        up_to_date_for_age: dosesValid >= dosesDue, doses_recorded: recorded, notes,
      };
    });
  }

  // Doses that match no series at all. Operates on the de-duplicated set.
  function findUnmatched(kept, schedule, idx) {
    const scheduleClasses = new Set(schedule.series.map((s) => s.product_class));
    const out = [];
    for (const imm of kept) {
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

  // --- FHIR input -----------------------------------------------------------

  // Reduce a FHIR R4 bundle (JSON string or parsed object) to the record the
  // engine consumes. Mirrors src/fhir.rs: completed immunisations only, first
  // coding with a code, occurrenceDateTime truncated to a date, the
  // UKCore-VaccinationProcedure SNOMED code, sorted ascending by date.
  const VACCINATION_PROCEDURE_EXT =
    'https://fhir.hl7.org.uk/StructureDefinition/Extension-UKCore-VaccinationProcedure';

  function parseFhirBundle(input) {
    const bundle = typeof input === 'string' ? JSON.parse(input) : input;
    let patient = null;
    const imms = [];
    for (const entry of bundle.entry || []) {
      const r = entry && entry.resource;
      if (!r) continue;
      if (r.resourceType === 'Patient') patient = r;
      else if (r.resourceType === 'Immunization') imms.push(r);
    }
    if (!patient) throw new Error('FHIR bundle has no Patient');

    const immunisations = imms
      .filter((i) => !i.status || i.status === 'completed')
      .map((i) => {
        const coding = ((i.vaccineCode && i.vaccineCode.coding) || []).find((c) => c.code);
        if (!coding) throw new Error('immunisation missing vaccineCode');
        const procExt = (i.extension || []).find((e) => e.url === VACCINATION_PROCEDURE_EXT);
        const procCoding = (((procExt && procExt.valueCodeableConcept && procExt.valueCodeableConcept.coding) || [])).find((c) => c.code);
        return {
          date: String(i.occurrenceDateTime).slice(0, 10),
          vaccine_code: coding.code,
          display: coding.display || null,
          dose_number: (i.protocolApplied || []).map((p) => p.doseNumberPositiveInt).find((n) => n != null) ?? null,
          procedure_code: (procCoding && procCoding.code) ?? null,
          procedure_display: (procCoding && procCoding.display) ?? null,
        };
      })
      .sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : 0));

    return {
      patientId: patient.id ?? null,
      dob: patient.birthDate,
      gender: patient.gender || null,
      immunisations,
    };
  }

  // --- Top level ------------------------------------------------------------

  function evaluate(record, schedule, productsFile, evaluatedAtStr) {
    const idx = makeProductIndex(productsFile);
    const evaluatedAt = parseDate(evaluatedAtStr);

    // 1. Drop duplicate echoes.
    const { kept, duplicates } = detectDuplicates(record.immunisations);

    // 2. Group series by product class, preserving first-seen order.
    const classOrder = [];
    const classSeries = new Map();
    for (const s of schedule.series) {
      if (!classSeries.has(s.product_class)) { classOrder.push(s.product_class); classSeries.set(s.product_class, []); }
      classSeries.get(s.product_class).push(s);
    }

    // 3. Evaluate each class group.
    let bySeries = [];
    for (const cls of classOrder) {
      const group = classSeries.get(cls);
      const classDoses = kept.filter((imm) => classFor(idx, imm.vaccine_code) === cls);
      bySeries = bySeries.concat(evaluateClassGroup(group, classDoses, record, evaluatedAt));
    }

    // 4. Restore schedule order.
    const order = new Map(schedule.series.map((s, i) => [s.id, i]));
    bySeries.sort((a, b) => order.get(a.series_id) - order.get(b.series_id));

    const status = aggregate(bySeries);
    const eligible = bySeries.filter((s) => s.eligible);
    const fullyVaccinated = eligible.length > 0 && eligible.every((s) => s.status === 'complete');

    return {
      status,
      fully_vaccinated: fullyVaccinated,
      evaluated_at: evaluatedAtStr,
      schedule_version: schedule.schedule.valid_from,
      by_series: bySeries,
      unmatched_doses: findUnmatched(kept, schedule, idx),
      duplicate_doses: duplicates,
      by_antigen: antigenCoverage(record, schedule, idx),
    };
  }

  return {
    evaluate, parseFhirBundle, makeProductIndex, classFor, antigensFor,
    ageOffsetToDate, parseAgeOffset, ageBetween, parseDate, fmtDate, addMonths, addDays,
  };
});
