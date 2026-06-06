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

use crate::error::EvaluationError;
use crate::fhir::{Immunisation, VaccinationRecord};
use crate::products::ProductMap;
use crate::schedule::{Eligibility, Schedule, Series};
use chrono::{Datelike, Months, NaiveDate};
use serde::Serialize;
use std::collections::HashSet;

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
    /// Which dose number in the series this was taken to be, if it landed within
    /// the expected count.
    pub assigned_dose_number: Option<u32>,
    /// True when the dose falls *within* the standard schedule (right age,
    /// interval met, not past any cutoff). False means "outside standard
    /// schedule" - the dose still happened, it just doesn't count. See §5.
    pub within_schedule: bool,
    /// When `within_schedule` is false, the specific reasons (too early, too
    /// late, interval too short, ...). Empty when the dose is fine.
    pub schedule_notes: Vec<String>,
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

/// Evaluate a vaccination record against a schedule.
pub fn evaluate(
    record: &VaccinationRecord,
    schedule: &Schedule,
    product_map: &ProductMap,
    evaluated_at: NaiveDate,
) -> Result<VaccinationStatus, EvaluationError> {
    // Evaluate each series independently. Order is preserved so the report reads
    // top-to-bottom in schedule (roughly chronological) order.
    let mut series_statuses = Vec::with_capacity(schedule.series.len());
    for series in &schedule.series {
        series_statuses.push(evaluate_series(series, record, product_map, evaluated_at));
    }

    let unmatched = find_unmatched_doses(record, schedule, product_map);
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
    })
}

/// Evaluate a single series: check eligibility, match doses by product class,
/// validate each dose, and classify completion and up-to-date-for-age.
fn evaluate_series(
    series: &Series,
    record: &VaccinationRecord,
    product_map: &ProductMap,
    evaluated_at: NaiveDate,
) -> SeriesStatus {
    // --- Eligibility -------------------------------------------------------
    // Decide first whether the patient is even in scope. An ineligible series is
    // NotApplicable and contributes nothing to the overall status.
    let elig = check_eligibility(&series.eligibility, record);
    let mut notes: Vec<String> = Vec::new();
    if let Some(note) = elig.note {
        notes.push(note);
    }

    let doses_expected = series.dose.len() as u32;

    // How many of the defined doses are *due* by the evaluation date? A dose is
    // due once its earliest_age (or target_age, if no earliest is set) has been
    // reached. This is what lets us say "up to date for age" rather than
    // penalising a child for a dose they are simply too young to have had.
    let doses_due = series
        .dose
        .iter()
        .filter(|dose| {
            let due_at = dose
                .earliest_age
                .unwrap_or(dose.target_age)
                .to_date(record.dob);
            due_at <= evaluated_at
        })
        .count() as u32;

    if !elig.eligible {
        // Not in scope: report the shape but mark NotApplicable and do no matching.
        return SeriesStatus {
            series_id: series.id.clone(),
            display_name: series.display_name.clone(),
            status: SeriesCompletionStatus::NotApplicable,
            eligible: false,
            eligibility_uncertain: elig.uncertain,
            doses_expected,
            doses_due,
            doses_valid: 0,
            up_to_date_for_age: true, // not applicable => nothing outstanding
            doses_recorded: Vec::new(),
            notes,
        };
    }

    // --- Dose matching (conformance) --------------------------------------
    // Conformance matching is by product class, not antigen overlap: a dose
    // belongs to this series only if the Green Book names its product for this
    // programme. This is what stops a 6-in-1 dose (which contains Hib) from
    // being dragged into the Hib/MenC booster series. See docs/adr/0001.
    let mut matched: Vec<&Immunisation> = record
        .immunisations
        .iter()
        .filter(|imm| {
            product_map.class_for(&imm.vaccine_code) == Some(series.product_class.as_str())
        })
        .collect();
    matched.sort_by_key(|i| i.date);

    // Walk the matched doses in date order, assigning each to the next expected
    // dose slot and checking it against that slot's age/interval rules.
    let mut recorded: Vec<RecordedDose> = Vec::new();
    let mut last_valid_date: Option<NaiveDate> = None;
    let mut next_dose_idx: usize = 0;

    for imm in &matched {
        let mut schedule_notes: Vec<String> = Vec::new();
        let mut within_schedule = true;
        let mut assigned: Option<u32> = None;

        if next_dose_idx >= series.dose.len() {
            // More doses of this class than the series defines.
            within_schedule = false;
            schedule_notes.push("extra dose beyond the expected count for this series".into());
        } else {
            let dose = &series.dose[next_dose_idx];
            assigned = Some(dose.number);

            // Too early: before the earliest age the dose may be given.
            if let Some(earliest) = dose.earliest_age {
                let earliest_date = earliest.to_date(record.dob);
                if imm.date < earliest_date {
                    within_schedule = false;
                    schedule_notes.push(format!(
                        "given before earliest_age {} ({}) - outside standard schedule",
                        earliest, earliest_date
                    ));
                }
            }
            // Too late: after a hard cutoff (e.g. rotavirus). Clinically this
            // dose must not count toward completion.
            if let Some(latest) = dose.latest_age {
                let latest_date = latest.to_date(record.dob);
                if imm.date > latest_date {
                    within_schedule = false;
                    schedule_notes.push(format!(
                        "given after latest_age {} ({}) - outside standard schedule",
                        latest, latest_date
                    ));
                }
            }
            // Interval too short since the previous *valid* dose.
            if let (Some(min_int), Some(prev)) = (dose.min_interval_from_previous, last_valid_date)
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
        }

        recorded.push(RecordedDose {
            date: imm.date,
            age_at_dose: age_between(record.dob, imm.date),
            vaccine_code: imm.vaccine_code.clone(),
            display: imm.display.clone(),
            assigned_dose_number: assigned,
            within_schedule,
            schedule_notes,
        });

        // Only an in-schedule dose advances the course and counts as the
        // baseline for the next dose's interval check.
        if within_schedule {
            last_valid_date = Some(imm.date);
            next_dose_idx += 1;
        }
    }

    let doses_valid = recorded.iter().filter(|d| d.within_schedule).count() as u32;

    let status = if doses_valid >= doses_expected {
        SeriesCompletionStatus::Complete
    } else if doses_valid > 0 {
        SeriesCompletionStatus::Partial
    } else {
        SeriesCompletionStatus::None
    };

    // Up to date for age: we have a valid dose for every dose due so far.
    let up_to_date_for_age = doses_valid >= doses_due;

    SeriesStatus {
        series_id: series.id.clone(),
        display_name: series.display_name.clone(),
        status,
        eligible: true,
        eligibility_uncertain: elig.uncertain,
        doses_expected,
        doses_due,
        doses_valid,
        up_to_date_for_age,
        doses_recorded: recorded,
        notes,
    }
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
/// reported rather than silently dropped.
fn find_unmatched_doses(
    record: &VaccinationRecord,
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
    for imm in &record.immunisations {
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
