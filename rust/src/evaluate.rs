//! The evaluation engine.
//!
//! Takes a parsed vaccination record, a schedule, and a product map, and answers
//! two distinct questions (see docs/adr/0001 and spec/standard.md §"Evaluation Logic"):
//!
//! 1. **Conformance** - did the patient receive the doses the Green Book named, at
//!    valid ages and intervals? Doses are matched to series by *product class*.
//! 2. **Status** - the headline, age-relative "is this patient up to date for their
//!    age?", plus a strict "have they had every dose the schedule defines?" flag.
//!
//! Antigen *coverage* ("what diseases are they protected against?") is a separate,
//! deferred computation and is not produced here yet.
//!
//! **Dose sequencing.** A product class can serve more than one series (e.g. `MMR`
//! → first-dose and second-dose series). Such a class is treated as one programme:
//! its doses are allocated across the class's series slots in **date order**, which
//! is the one signal a human can't mis-key. The recorded `protocolApplied` dose
//! number and any dose encoded in the SNOMED procedure code are **cross-checks**
//! that raise a flag on disagreement, never overriding the date-based allocation.
//!
//! **Duplicates ("echoes").** The same physical vaccination is often recorded twice
//! from different systems with different dates. Where two records share the same
//! procedure code they are taken to be the same act; the earliest is kept and the
//! rest reported as duplicates rather than counted as extra doses.

use crate::error::EvaluationError;
use crate::fhir::{Immunisation, VaccinationRecord};
use crate::products::ProductMap;
use crate::schedule::{Dose, Eligibility, Schedule, Series};
use chrono::{Datelike, Months, NaiveDate};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// The full result of evaluating one patient against one schedule.
#[derive(Debug, Clone, Serialize)]
pub struct VaccinationStatus {
    /// The headline, age-relative determination (see [`OverallStatus`]).
    pub status: OverallStatus,
    /// Strict, age-independent flag: every series the patient is eligible for is
    /// `Complete` (every dose at every age received and valid). A correctly
    /// vaccinated 6-month-old is up-to-date but *not* yet `fully_vaccinated`.
    pub fully_vaccinated: bool,
    /// The date the patient was evaluated against (today, or a supplied date).
    pub evaluated_at: NaiveDate,
    /// `valid_from` of the schedule version used.
    pub schedule_version: NaiveDate,
    /// One entry per series in the schedule, in schedule order.
    pub by_series: Vec<SeriesStatus>,
    /// Recorded doses that matched no series at all - either an unknown product
    /// code or a known product whose class no series in this schedule asks for.
    /// These would otherwise vanish silently from the report.
    pub unmatched_doses: Vec<UnmatchedDose>,
    /// Recorded doses dropped as likely duplicates ("echoes") of an earlier dose
    /// with the same procedure code. Reported, not counted.
    pub duplicate_doses: Vec<DuplicateDose>,
}

/// The headline answer to "is this patient correctly vaccinated for their age?".
///
/// This is deliberately distinct from the strict [`VaccinationStatus::fully_vaccinated`]
/// flag: that asks the age-independent "have they had *everything* the schedule
/// ever defines?". See spec/standard.md §"Overall status".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverallStatus {
    /// Every dose that is *due* by `evaluated_at` has been received and is valid.
    /// Doses not yet due are not held against the patient, so a perfectly
    /// on-schedule infant lands here rather than looking "partially vaccinated".
    UpToDateForAge,
    /// At least one dose that is already due is missing (or only satisfied by a
    /// dose given outside the standard schedule). This is the case the old
    /// `PartiallyVaccinated` used to cover.
    BehindForAge,
    /// No valid doses recorded at all, despite some being due.
    Unvaccinated,
    /// Cannot determine - e.g. the patient is eligible for no series.
    Unknown,
}

/// Per-series outcome.
#[derive(Debug, Clone, Serialize)]
pub struct SeriesStatus {
    pub series_id: String,
    pub display_name: String,
    pub status: SeriesCompletionStatus,
    /// Whether the patient is eligible for this series at all. When false, the
    /// completion status is `NotApplicable` and the series is excluded from the
    /// overall determination.
    pub eligible: bool,
    /// True when eligibility could not be decided for certain - currently only
    /// when a sex-restricted series meets a patient whose `gender` is
    /// `other`/`unknown`. The patient is treated as eligible, but flagged.
    pub eligibility_uncertain: bool,
    /// Total doses the series defines.
    pub doses_expected: u32,
    /// Of those, how many are *due* by `evaluated_at` (earliest_age reached).
    pub doses_due: u32,
    /// How many recorded doses were valid (within the standard schedule).
    pub doses_valid: u32,
    /// True when the patient has a valid dose for every dose due so far. This is
    /// the per-series input to the headline `UpToDateForAge` status.
    pub up_to_date_for_age: bool,
    /// Every recorded dose matched to this series, valid or not.
    pub doses_recorded: Vec<RecordedDose>,
    /// Free-text notes about the series as a whole (e.g. eligibility uncertainty).
    pub notes: Vec<String>,
}

/// A series' standing against the doses it defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesCompletionStatus {
    /// All expected doses received and valid.
    Complete,
    /// At least one valid dose, but fewer than expected.
    Partial,
    /// No valid doses.
    None,
    /// Patient not eligible for this series.
    NotApplicable,
}

/// A single recorded vaccination event, assigned to a series.
#[derive(Debug, Clone, Serialize)]
pub struct RecordedDose {
    pub date: NaiveDate,
    /// Human-readable age at administration, e.g. "3mo 2w 6d".
    pub age_at_dose: String,
    pub vaccine_code: String,
    pub display: Option<String>,
    /// Which dose number in the series this was taken to be (by date order), if it
    /// landed within the expected count.
    pub assigned_dose_number: Option<u32>,
    /// True when the dose falls *within* the standard schedule (right age,
    /// interval met, not past any cutoff). False means "outside standard
    /// schedule" - the dose still happened, it just doesn't count. See §5.
    pub within_schedule: bool,
    /// When `within_schedule` is false, the specific reasons (too early, too
    /// late, interval too short, ...). Empty when the dose is fine.
    pub schedule_notes: Vec<String>,
    /// Soft cross-check warnings that do *not* affect validity - notably when the
    /// recorded dose number (FHIR `protocolApplied` or the SNOMED procedure code)
    /// disagrees with the position derived from dates. Surfaced for human review.
    pub flags: Vec<String>,
}

/// A recorded dose that belongs to no series in the loaded schedule.
#[derive(Debug, Clone, Serialize)]
pub struct UnmatchedDose {
    pub date: NaiveDate,
    pub vaccine_code: String,
    pub display: Option<String>,
    /// Why it matched nothing: unknown product code, or a known product whose
    /// class no series in this schedule version asks for.
    pub reason: String,
}

/// A recorded dose dropped as a likely duplicate of an earlier dose.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateDose {
    pub date: NaiveDate,
    pub vaccine_code: String,
    pub display: Option<String>,
    /// The procedure code shared with the kept dose (the duplicate signal).
    pub procedure_code: Option<String>,
    /// The date of the earlier dose this one duplicates.
    pub duplicate_of: NaiveDate,
}

/// Evaluate a vaccination record against a schedule.
pub fn evaluate(
    record: &VaccinationRecord,
    schedule: &Schedule,
    product_map: &ProductMap,
    evaluated_at: NaiveDate,
) -> Result<VaccinationStatus, EvaluationError> {
    // 1. Drop duplicate "echoes" before anything else, so the same physical jab
    //    recorded twice from two systems isn't counted as two doses.
    let (kept, duplicate_doses) = detect_duplicates(&record.immunisations);

    // 2. Group the schedule's series by product class, preserving the order in
    //    which each class first appears. A class with several series (e.g. MMR)
    //    is evaluated as one programme so its doses are allocated across the
    //    series rather than matched against each independently.
    let mut class_order: Vec<&str> = Vec::new();
    let mut class_series: HashMap<&str, Vec<&Series>> = HashMap::new();
    for s in &schedule.series {
        let class = s.product_class.as_str();
        if !class_series.contains_key(class) {
            class_order.push(class);
        }
        class_series.entry(class).or_default().push(s);
    }

    // 3. Evaluate each class group; collect the per-series results.
    let mut series_statuses: Vec<SeriesStatus> = Vec::with_capacity(schedule.series.len());
    for class in &class_order {
        let group = &class_series[*class];
        let class_doses: Vec<&Immunisation> = kept
            .iter()
            .copied()
            .filter(|imm| product_map.class_for(&imm.vaccine_code) == Some(*class))
            .collect();
        series_statuses.extend(evaluate_class_group(
            group,
            &class_doses,
            record,
            evaluated_at,
        ));
    }

    // 4. Restore original schedule order for the report.
    let order: HashMap<&str, usize> = schedule
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    series_statuses.sort_by_key(|st| order[st.series_id.as_str()]);

    let unmatched = find_unmatched_doses(&kept, schedule, product_map);
    let status = aggregate(&series_statuses);

    // "Fully vaccinated" is the strict sense: every series the patient is
    // eligible for is Complete, regardless of age. Distinct from the headline.
    let eligible: Vec<&SeriesStatus> = series_statuses.iter().filter(|s| s.eligible).collect();
    let fully_vaccinated = !eligible.is_empty()
        && eligible
            .iter()
            .all(|s| s.status == SeriesCompletionStatus::Complete);

    Ok(VaccinationStatus {
        status,
        fully_vaccinated,
        evaluated_at,
        schedule_version: schedule.schedule.valid_from,
        by_series: series_statuses,
        unmatched_doses: unmatched,
        duplicate_doses,
    })
}

/// Detect duplicate "echoes": records sharing a procedure code are the same act.
/// The earliest is kept; the rest are reported as duplicates. Records with no
/// procedure code are always kept (we have no duplicate signal for them).
///
/// `immunisations` is assumed date-sorted (the FHIR parser sorts), so the first
/// occurrence of a procedure code is the earliest.
fn detect_duplicates(immunisations: &[Immunisation]) -> (Vec<&Immunisation>, Vec<DuplicateDose>) {
    let mut seen: HashMap<&str, NaiveDate> = HashMap::new();
    let mut kept: Vec<&Immunisation> = Vec::new();
    let mut duplicates: Vec<DuplicateDose> = Vec::new();

    for imm in immunisations {
        match imm.procedure_code.as_deref() {
            Some(code) => match seen.get(code) {
                Some(first_date) => duplicates.push(DuplicateDose {
                    date: imm.date,
                    vaccine_code: imm.vaccine_code.clone(),
                    display: imm.display.clone(),
                    procedure_code: imm.procedure_code.clone(),
                    duplicate_of: *first_date,
                }),
                None => {
                    seen.insert(code, imm.date);
                    kept.push(imm);
                }
            },
            None => kept.push(imm),
        }
    }
    (kept, duplicates)
}

/// Evaluate all series that share one product class as a single programme,
/// allocating the class's recorded doses across the series' dose slots by date.
fn evaluate_class_group(
    group: &[&Series],
    class_doses: &[&Immunisation],
    record: &VaccinationRecord,
    evaluated_at: NaiveDate,
) -> Vec<SeriesStatus> {
    // Eligibility per series in the group.
    let eligs: Vec<EligibilityOutcome> = group
        .iter()
        .map(|s| check_eligibility(&s.eligibility, record))
        .collect();

    // Build the ordered list of dose slots from the *eligible* series, sorted by
    // the date each dose is targeted at. Each slot remembers which series it
    // belongs to (index into `group`).
    struct Slot<'a> {
        gi: usize,
        dose: &'a Dose,
    }
    let mut slots: Vec<Slot> = Vec::new();
    for (gi, s) in group.iter().enumerate() {
        if eligs[gi].eligible {
            for dose in &s.dose {
                slots.push(Slot { gi, dose });
            }
        }
    }
    slots.sort_by_key(|sl| sl.dose.target_age.to_date(record.dob));

    // The series an "extra" dose (beyond all slots) is attached to: the last
    // eligible series in the group.
    let last_eligible_gi = (0..group.len()).rev().find(|&gi| eligs[gi].eligible);

    // Walk the class's doses in date order, assigning each to the next slot.
    let mut recorded_by_gi: Vec<Vec<RecordedDose>> = vec![Vec::new(); group.len()];
    let mut last_valid_date: Option<NaiveDate> = None;

    for (i, imm) in class_doses.iter().enumerate() {
        let mut schedule_notes: Vec<String> = Vec::new();
        let mut flags: Vec<String> = Vec::new();
        let mut within_schedule = true;
        let mut assigned: Option<u32> = None;

        let target_gi = if i < slots.len() {
            let slot = &slots[i];
            assigned = Some(slot.dose.number);

            // Too early.
            if let Some(earliest) = slot.dose.earliest_age {
                let earliest_date = earliest.to_date(record.dob);
                if imm.date < earliest_date {
                    within_schedule = false;
                    schedule_notes.push(format!(
                        "given before earliest_age {} ({}) - outside standard schedule",
                        earliest, earliest_date
                    ));
                }
            }
            // Too late (hard cutoff).
            if let Some(latest) = slot.dose.latest_age {
                let latest_date = latest.to_date(record.dob);
                if imm.date > latest_date {
                    within_schedule = false;
                    schedule_notes.push(format!(
                        "given after latest_age {} ({}) - outside standard schedule",
                        latest, latest_date
                    ));
                }
            }
            // Interval too short since the previous *valid* dose in this programme.
            if let (Some(min_int), Some(prev)) =
                (slot.dose.min_interval_from_previous, last_valid_date)
            {
                let earliest_by_interval = min_int.to_date(prev);
                if imm.date < earliest_by_interval {
                    within_schedule = false;
                    schedule_notes.push(format!(
                        "interval from previous dose < {} (needs to be on/after {}) - outside standard schedule",
                        min_int, earliest_by_interval
                    ));
                }
            }

            // Cross-check the date-derived dose number against the recorded
            // signals. Disagreement is flagged for review, never overriding.
            if let Some(recorded_n) = imm.dose_number {
                if recorded_n != slot.dose.number {
                    flags.push(format!(
                        "recorded dose number {} disagrees with position-by-date (dose {}) - sequence may be mis-keyed",
                        recorded_n, slot.dose.number
                    ));
                }
            }
            if let Some(proc_n) = dose_from_procedure(imm.procedure_display.as_deref()) {
                if proc_n != slot.dose.number {
                    flags.push(format!(
                        "procedure code indicates dose {} but by date this is dose {}",
                        proc_n, slot.dose.number
                    ));
                }
            }

            slot.gi
        } else {
            // More doses of this class than the programme defines.
            within_schedule = false;
            schedule_notes.push("extra dose beyond the expected count for this class".into());
            // Attach to the last eligible series so it is still reported.
            match last_eligible_gi {
                Some(gi) => gi,
                None => continue, // no eligible series in the group; ignore
            }
        };

        recorded_by_gi[target_gi].push(RecordedDose {
            date: imm.date,
            age_at_dose: age_between(record.dob, imm.date),
            vaccine_code: imm.vaccine_code.clone(),
            display: imm.display.clone(),
            assigned_dose_number: assigned,
            within_schedule,
            schedule_notes,
            flags,
        });

        if within_schedule {
            last_valid_date = Some(imm.date);
        }
    }

    // Build a SeriesStatus for each series in the group.
    group
        .iter()
        .enumerate()
        .map(|(gi, s)| {
            let elig = &eligs[gi];
            let notes: Vec<String> = elig.note.clone().into_iter().collect();
            let doses_expected = s.dose.len() as u32;
            let doses_due = doses_due_for(s, record.dob, evaluated_at);

            if !elig.eligible {
                return SeriesStatus {
                    series_id: s.id.clone(),
                    display_name: s.display_name.clone(),
                    status: SeriesCompletionStatus::NotApplicable,
                    eligible: false,
                    eligibility_uncertain: elig.uncertain,
                    doses_expected,
                    doses_due,
                    doses_valid: 0,
                    up_to_date_for_age: true,
                    doses_recorded: Vec::new(),
                    notes,
                };
            }

            let recorded = std::mem::take(&mut recorded_by_gi[gi]);
            let doses_valid = recorded.iter().filter(|d| d.within_schedule).count() as u32;
            let status = if doses_valid >= doses_expected {
                SeriesCompletionStatus::Complete
            } else if doses_valid > 0 {
                SeriesCompletionStatus::Partial
            } else {
                SeriesCompletionStatus::None
            };

            SeriesStatus {
                series_id: s.id.clone(),
                display_name: s.display_name.clone(),
                status,
                eligible: true,
                eligibility_uncertain: elig.uncertain,
                doses_expected,
                doses_due,
                doses_valid,
                up_to_date_for_age: doses_valid >= doses_due,
                doses_recorded: recorded,
                notes,
            }
        })
        .collect()
}

/// How many of a series' defined doses are *due* by the evaluation date. A dose
/// is due once its `earliest_age` (or `target_age`, if no earliest is set) is
/// reached.
fn doses_due_for(series: &Series, dob: NaiveDate, evaluated_at: NaiveDate) -> u32 {
    series
        .dose
        .iter()
        .filter(|dose| {
            let due_at = dose.earliest_age.unwrap_or(dose.target_age).to_date(dob);
            due_at <= evaluated_at
        })
        .count() as u32
}

/// Parse a dose number from a SNOMED procedure code's display text, e.g.
/// "Administration of second dose of ... vaccine (procedure)" -> 2. The *first*
/// dose is often recorded with the generic administration code (no "first" in
/// it), so a `None` here just means "no explicit dose stated".
fn dose_from_procedure(display: Option<&str>) -> Option<u32> {
    let d = display?.to_ascii_lowercase();
    for (word, n) in [
        ("first", 1u32),
        ("second", 2),
        ("third", 3),
        ("fourth", 4),
        ("fifth", 5),
    ] {
        if d.contains(&format!("{} dose", word)) {
            return Some(n);
        }
    }
    None
}

/// Outcome of an eligibility check for one series.
struct EligibilityOutcome {
    eligible: bool,
    /// Eligibility could not be decided for certain (gender other/unknown on a
    /// sex-restricted series); we default to eligible and flag it.
    uncertain: bool,
    /// Optional explanation to surface to the reader.
    note: Option<String>,
}

/// Decide whether a patient is eligible for a series.
///
/// Rules (spec/standard.md §"Eligibility check"):
/// - `population` "all" is universally eligible, subject to any
///   `male_born_on_or_after` cohort restriction on males.
/// - `population` "female"/"male" requires the patient's sex to match.
/// - When the FHIR `gender` is `other`/`unknown`, we cannot apply a sex rule, so
///   we treat the patient as eligible and set `uncertain` so the determination's
///   dependence on a missing data point is visible.
fn check_eligibility(eligibility: &Eligibility, record: &VaccinationRecord) -> EligibilityOutcome {
    let gender = record.gender.as_deref();
    let is_male = gender == Some("male");
    let is_female = gender == Some("female");
    // "other", "unknown", or absent: sex-based rules cannot be applied.
    let sex_unknown = !is_male && !is_female;

    match eligibility.population.as_str() {
        "female" => {
            if is_female {
                EligibilityOutcome {
                    eligible: true,
                    uncertain: false,
                    note: None,
                }
            } else if sex_unknown {
                EligibilityOutcome {
                    eligible: true,
                    uncertain: true,
                    note: Some(
                        "female-only series; patient gender is other/unknown, treated as eligible"
                            .into(),
                    ),
                }
            } else {
                EligibilityOutcome {
                    eligible: false,
                    uncertain: false,
                    note: None,
                }
            }
        }
        "male" => {
            if is_male {
                male_cohort_outcome(eligibility, record)
            } else if sex_unknown {
                EligibilityOutcome {
                    eligible: true,
                    uncertain: true,
                    note: Some(
                        "male-only series; patient gender is other/unknown, treated as eligible"
                            .into(),
                    ),
                }
            } else {
                EligibilityOutcome {
                    eligible: false,
                    uncertain: false,
                    note: None,
                }
            }
        }
        // "all" (or anything else we don't restrict on): everyone is eligible,
        // except that a male-cohort cutoff can still exclude males born too early
        // (e.g. HPV, offered to all girls but only to boys born on/after a date).
        _ => {
            if is_male {
                male_cohort_outcome(eligibility, record)
            } else if sex_unknown && eligibility.male_born_on_or_after.is_some() {
                // A male born before the cutoff would be ineligible, so an
                // unknown-gender patient's eligibility genuinely depends on the
                // missing sex/DOB-cohort fact.
                EligibilityOutcome {
                    eligible: true,
                    uncertain: true,
                    note: Some(
                        "series restricts males by birth cohort; patient gender is other/unknown, treated as eligible"
                            .into(),
                    ),
                }
            } else {
                EligibilityOutcome {
                    eligible: true,
                    uncertain: false,
                    note: None,
                }
            }
        }
    }
}

/// Apply a `male_born_on_or_after` cohort cutoff to a male patient.
fn male_cohort_outcome(
    eligibility: &Eligibility,
    record: &VaccinationRecord,
) -> EligibilityOutcome {
    match eligibility.male_born_on_or_after {
        Some(cutoff) if record.dob < cutoff => EligibilityOutcome {
            eligible: false,
            uncertain: false,
            note: Some(format!(
                "male born before {} - outside the eligible birth cohort",
                cutoff
            )),
        },
        _ => EligibilityOutcome {
            eligible: true,
            uncertain: false,
            note: None,
        },
    }
}

/// Collect recorded doses that belong to no series in the schedule, so they are
/// reported rather than silently dropped. Operates on the de-duplicated set.
fn find_unmatched_doses(
    kept: &[&Immunisation],
    schedule: &Schedule,
    product_map: &ProductMap,
) -> Vec<UnmatchedDose> {
    // The set of product classes any series in this schedule actually asks for.
    let schedule_classes: HashSet<&str> = schedule
        .series
        .iter()
        .map(|s| s.product_class.as_str())
        .collect();

    let mut unmatched = Vec::new();
    for imm in kept {
        match product_map.class_for(&imm.vaccine_code) {
            // Code isn't in the product map at all.
            None => unmatched.push(UnmatchedDose {
                date: imm.date,
                vaccine_code: imm.vaccine_code.clone(),
                display: imm.display.clone(),
                reason: "unknown product code (not in the product map)".into(),
            }),
            // Known product, but no series in this schedule version uses its
            // class (e.g. a 5-in-1 dose against the 2026 6-in-1 schedule).
            Some(class) if !schedule_classes.contains(class) => unmatched.push(UnmatchedDose {
                date: imm.date,
                vaccine_code: imm.vaccine_code.clone(),
                display: imm.display.clone(),
                reason: format!(
                    "product class \"{}\" has no series in this schedule version",
                    class
                ),
            }),
            // Otherwise it was handled by a series.
            Some(_) => {}
        }
    }
    unmatched
}

/// Roll the per-series outcomes up into the headline age-relative status.
fn aggregate(series: &[SeriesStatus]) -> OverallStatus {
    // Only series the patient is eligible for count toward the overall picture.
    let applicable: Vec<&SeriesStatus> = series.iter().filter(|s| s.eligible).collect();

    if applicable.is_empty() {
        return OverallStatus::Unknown;
    }

    let total_valid: u32 = applicable.iter().map(|s| s.doses_valid).sum();
    let total_due: u32 = applicable.iter().map(|s| s.doses_due).sum();
    let all_up_to_date = applicable.iter().all(|s| s.up_to_date_for_age);

    if total_valid == 0 {
        // No valid doses anywhere. If nothing was due yet (e.g. a newborn) the
        // patient is trivially up to date; otherwise they are unvaccinated.
        if total_due == 0 {
            OverallStatus::UpToDateForAge
        } else {
            OverallStatus::Unvaccinated
        }
    } else if all_up_to_date {
        OverallStatus::UpToDateForAge
    } else {
        OverallStatus::BehindForAge
    }
}

/// Render the gap between two dates as a compact human age string like
/// "3mo 2w 6d". Used for "age at dose" in the report. Calendar-aware: months are
/// real calendar months, then remaining days are split into weeks and days.
fn age_between(dob: NaiveDate, when: NaiveDate) -> String {
    let mut years: u32 = (when.year() - dob.year()) as u32;
    let mut after_years = dob.with_year(dob.year() + years as i32).unwrap_or(dob);
    if after_years > when && years > 0 {
        years -= 1;
        after_years = dob.with_year(dob.year() + years as i32).unwrap_or(dob);
    }
    let mut months = 0u32;
    let mut cursor = after_years;
    while let Some(next) = cursor.checked_add_months(Months::new(1)) {
        if next > when {
            break;
        }
        cursor = next;
        months += 1;
    }
    let days_after_months = (when - cursor).num_days().max(0) as u32;
    let weeks = days_after_months / 7;
    let leftover = days_after_months % 7;
    let mut parts = Vec::new();
    if years > 0 {
        parts.push(format!("{}y", years));
    }
    if months > 0 {
        parts.push(format!("{}mo", months));
    }
    if weeks > 0 {
        parts.push(format!("{}w", weeks));
    }
    if leftover > 0 || parts.is_empty() {
        parts.push(format!("{}d", leftover));
    }
    parts.join(" ")
}
