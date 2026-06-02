use crate::error::EvaluationError;
use crate::fhir::{Immunisation, VaccinationRecord};
use crate::products::ProductMap;
use crate::schedule::{Schedule, Series};
use chrono::{Datelike, Months, NaiveDate};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct VaccinationStatus {
    pub overall: OverallStatus,
    pub evaluated_at: NaiveDate,
    pub schedule_version: NaiveDate,
    pub by_series: Vec<SeriesStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverallStatus {
    FullyVaccinated,
    PartiallyVaccinated,
    Unvaccinated,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesStatus {
    pub series_id: String,
    pub display_name: String,
    pub status: SeriesCompletionStatus,
    pub doses_expected: u32,
    pub doses_valid: u32,
    pub doses_recorded: Vec<RecordedDose>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesCompletionStatus {
    Complete,
    Partial,
    None,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordedDose {
    pub date: NaiveDate,
    pub age_at_dose: String,
    pub vaccine_code: String,
    pub display: Option<String>,
    pub assigned_dose_number: Option<u32>,
    pub valid: bool,
    pub validity_reasons: Vec<String>,
}

pub fn evaluate(
    record: &VaccinationRecord,
    schedule: &Schedule,
    product_map: &ProductMap,
    evaluated_at: NaiveDate,
) -> Result<VaccinationStatus, EvaluationError> {
    let mut series_statuses = Vec::with_capacity(schedule.series.len());
    for series in &schedule.series {
        series_statuses.push(evaluate_series(series, record, product_map)?);
    }
    let overall = aggregate(&series_statuses);
    Ok(VaccinationStatus {
        overall,
        evaluated_at,
        schedule_version: schedule.schedule.valid_from,
        by_series: series_statuses,
    })
}

fn evaluate_series(
    series: &Series,
    record: &VaccinationRecord,
    product_map: &ProductMap,
) -> Result<SeriesStatus, EvaluationError> {
    let series_antigens: HashSet<&str> = series.antigens.iter().map(String::as_str).collect();

    let mut matched: Vec<&Immunisation> = record
        .immunisations
        .iter()
        .filter(|imm| {
            product_map
                .antigens_for(&imm.vaccine_code)
                .map(|ants| ants.iter().any(|a| series_antigens.contains(a.as_str())))
                .unwrap_or(false)
        })
        .collect();
    matched.sort_by_key(|i| i.date);

    let doses_expected = series.dose.len() as u32;

    let mut recorded: Vec<RecordedDose> = Vec::new();
    let mut last_valid_date: Option<NaiveDate> = None;
    let mut next_dose_idx: usize = 0;

    for imm in &matched {
        let mut reasons: Vec<String> = Vec::new();
        let mut valid = true;
        let mut assigned: Option<u32> = None;

        if next_dose_idx >= series.dose.len() {
            valid = false;
            reasons.push("extra dose beyond expected schedule".into());
        } else {
            let dose = &series.dose[next_dose_idx];
            assigned = Some(dose.number);

            if let Some(earliest) = dose.earliest_age {
                let earliest_date = earliest.to_date(record.dob);
                if imm.date < earliest_date {
                    valid = false;
                    reasons.push(format!(
                        "given before earliest_age {} (would be {})",
                        earliest, earliest_date
                    ));
                }
            }
            if let Some(latest) = dose.latest_age {
                let latest_date = latest.to_date(record.dob);
                if imm.date > latest_date {
                    valid = false;
                    reasons.push(format!(
                        "given after latest_age {} (would be {})",
                        latest, latest_date
                    ));
                }
            }
            if let (Some(min_int), Some(prev)) = (dose.min_interval_from_previous, last_valid_date)
            {
                let earliest_by_interval = min_int.to_date(prev);
                if imm.date < earliest_by_interval {
                    valid = false;
                    reasons.push(format!(
                        "interval from previous dose < {} (would need to be on/after {})",
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
            valid,
            validity_reasons: reasons,
        });

        if valid {
            last_valid_date = Some(imm.date);
            next_dose_idx += 1;
        }
    }

    let doses_valid = recorded.iter().filter(|d| d.valid).count() as u32;

    let status = if doses_valid >= doses_expected {
        SeriesCompletionStatus::Complete
    } else if doses_valid > 0 {
        SeriesCompletionStatus::Partial
    } else {
        SeriesCompletionStatus::None
    };

    Ok(SeriesStatus {
        series_id: series.id.clone(),
        display_name: series.display_name.clone(),
        status,
        doses_expected,
        doses_valid,
        doses_recorded: recorded,
    })
}

fn aggregate(series: &[SeriesStatus]) -> OverallStatus {
    let applicable: Vec<&SeriesStatus> = series
        .iter()
        .filter(|s| s.status != SeriesCompletionStatus::NotApplicable)
        .collect();

    if applicable.is_empty() {
        return OverallStatus::Unknown;
    }

    let all_complete = applicable
        .iter()
        .all(|s| s.status == SeriesCompletionStatus::Complete);
    let all_none = applicable
        .iter()
        .all(|s| s.status == SeriesCompletionStatus::None);
    let any_doses = applicable.iter().any(|s| s.doses_valid > 0);

    if all_complete {
        OverallStatus::FullyVaccinated
    } else if all_none {
        OverallStatus::Unvaccinated
    } else if any_doses {
        OverallStatus::PartiallyVaccinated
    } else {
        OverallStatus::Unvaccinated
    }
}

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
