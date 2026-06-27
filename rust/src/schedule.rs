use crate::age::AgeOffset;
use crate::error::ScheduleError;
use chrono::{Days, NaiveDate};
use serde::Deserialize;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Schedule {
    pub jurisdiction: Jurisdiction,
    pub schedule: ScheduleMeta,
    #[serde(default)]
    pub series: Vec<Series>,
    #[serde(default)]
    pub antigen: Vec<Antigen>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Jurisdiction {
    pub country: String,
    pub country_name: Option<String>,
    pub schedule_authority: String,
    pub product_coding_system: String,
    pub language: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleMeta {
    pub valid_from: NaiveDate,
    #[serde(default)]
    pub valid_to: Option<NaiveDate>,
    pub supersedes: Option<NaiveDate>,
    pub source_document: String,
    pub source_url: String,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Series {
    pub id: String,
    pub display_name: String,
    pub description: String,
    /// The product class whose doses conform to this series (e.g. "6-in-1").
    /// Conformance matching is by class, not antigen overlap — see
    /// spec/conformance-vs-coverage.md and the product map's `product_class` field.
    pub product_class: String,
    pub antigens: Vec<String>,
    pub eligibility: Eligibility,
    pub dose: Vec<Dose>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Eligibility {
    pub population: String,
    #[serde(default)]
    pub male_born_on_or_after: Option<NaiveDate>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dose {
    pub number: u32,
    pub target_age: AgeOffset,
    pub earliest_age: Option<AgeOffset>,
    pub latest_age: Option<AgeOffset>,
    pub min_interval_from_previous: Option<AgeOffset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Antigen {
    pub id: String,
    pub display_name: String,
    pub snomed_concept: String,
    #[serde(default)]
    pub snomed_description: Option<String>,
}

pub fn load_schedule(path: &Path) -> Result<Schedule, ScheduleError> {
    let raw = fs::read_to_string(path)?;
    let schedule: Schedule = toml::from_str(&raw)?;
    validate_referential_integrity(&schedule)?;
    Ok(schedule)
}

#[derive(Debug, Clone)]
pub struct ScheduleVersion {
    pub path: PathBuf,
    pub schedule: Schedule,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScheduleVersionSummary {
    pub path: String,
    pub valid_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub source_document: String,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScheduleSelection {
    pub country: String,
    pub dob: NaiveDate,
    pub evaluated_at: NaiveDate,
    pub rule: String,
    pub versions: Vec<ScheduleVersionSummary>,
}

#[derive(Debug, Clone)]
pub struct HistoricalSchedule {
    pub schedule: Schedule,
    pub selection: ScheduleSelection,
}

pub fn load_schedule_versions(
    rules_dir: &Path,
    country: &str,
) -> Result<Vec<ScheduleVersion>, ScheduleError> {
    let prefix = format!("schedule-{}-", country.to_ascii_lowercase());
    let mut versions = Vec::new();

    for entry in fs::read_dir(rules_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !file_name.starts_with(&prefix) || !file_name.ends_with(".toml") {
            continue;
        }

        let schedule = load_schedule(&path)?;
        if !schedule.jurisdiction.country.eq_ignore_ascii_case(country) {
            continue;
        }
        versions.push(ScheduleVersion {
            path,
            schedule,
            effective_to: None,
        });
    }

    if versions.is_empty() {
        return Err(ScheduleError::NoScheduleVersions {
            country: country.to_string(),
            dir: rules_dir.display().to_string(),
        });
    }

    versions.sort_by_key(|v| v.schedule.schedule.valid_from);
    validate_and_fill_effective_ranges(&mut versions)?;
    Ok(versions)
}

pub fn load_effective_schedule_for_date(
    rules_dir: &Path,
    country: &str,
    dob: NaiveDate,
    evaluated_at: NaiveDate,
) -> Result<HistoricalSchedule, ScheduleError> {
    let versions = load_schedule_versions(rules_dir, country)?;
    effective_schedule_for_versions(&versions, country, dob, evaluated_at)
}

pub fn effective_schedule_for_versions(
    versions: &[ScheduleVersion],
    country: &str,
    dob: NaiveDate,
    evaluated_at: NaiveDate,
) -> Result<HistoricalSchedule, ScheduleError> {
    let evaluation_version = version_for_date(versions, country, evaluated_at)?;
    let mut effective = evaluation_version.schedule.clone();
    effective.schedule.change_summary =
        Some("Effective historical schedule assembled by dose due date.".into());
    effective.series.clear();
    effective.antigen.clear();

    let mut index_by_id: HashMap<String, usize> = HashMap::new();
    let mut antigen_by_id: BTreeMap<String, Antigen> = BTreeMap::new();
    let mut used_counts: HashMap<NaiveDate, usize> = HashMap::new();

    for version in versions {
        for series in &version.schedule.series {
            let mut selected = Vec::new();
            for dose in &series.dose {
                let due_at = dose_due_at(dose, dob);
                let selector_date = due_at.min(evaluated_at);
                let selected_version = version_for_date(versions, country, selector_date)?;
                if selected_version.schedule.schedule.valid_from
                    == version.schedule.schedule.valid_from
                {
                    selected.push(dose.clone());
                    *used_counts
                        .entry(version.schedule.schedule.valid_from)
                        .or_default() += 1;
                }
            }

            if selected.is_empty() {
                continue;
            }

            selected.sort_by_key(|dose| dose.target_age.to_date(dob));
            let mut selected_series = series.clone();
            selected_series.dose = selected;
            for antigen in &version.schedule.antigen {
                antigen_by_id
                    .entry(antigen.id.clone())
                    .or_insert_with(|| antigen.clone());
            }
            insert_effective_series(
                &mut effective.series,
                &mut index_by_id,
                selected_series,
                version.schedule.schedule.valid_from,
                dob,
            );
        }
    }
    effective.antigen = antigen_by_id.into_values().collect();

    let versions_used = versions
        .iter()
        .filter(|v| used_counts.contains_key(&v.schedule.schedule.valid_from))
        .map(|v| ScheduleVersionSummary {
            path: v.path.display().to_string(),
            valid_from: v.schedule.schedule.valid_from,
            effective_to: v.effective_to,
            source_document: v.schedule.schedule.source_document.clone(),
            change_summary: v.schedule.schedule.change_summary.clone(),
        })
        .collect();

    Ok(HistoricalSchedule {
        schedule: effective,
        selection: ScheduleSelection {
            country: country.to_string(),
            dob,
            evaluated_at,
            rule: "dose slots are selected from the schedule version in force when each dose first became due; not-yet-due slots are projected from the version in force on evaluated_at".into(),
            versions: versions_used,
        },
    })
}

fn validate_and_fill_effective_ranges(
    versions: &mut [ScheduleVersion],
) -> Result<(), ScheduleError> {
    for i in 0..versions.len() {
        let valid_from = versions[i].schedule.schedule.valid_from;
        if i > 0 && versions[i - 1].schedule.schedule.valid_from == valid_from {
            return Err(ScheduleError::DuplicateScheduleVersion(
                valid_from.to_string(),
            ));
        }

        if let Some(valid_to) = versions[i].schedule.schedule.valid_to {
            if valid_to < valid_from {
                return Err(ScheduleError::InvalidScheduleRange {
                    valid_from: valid_from.to_string(),
                });
            }
        }

        let next_valid_from = versions.get(i + 1).map(|v| v.schedule.schedule.valid_from);
        let explicit_to_exclusive = versions[i]
            .schedule
            .schedule
            .valid_to
            .and_then(|d| d.checked_add_days(Days::new(1)));

        if let (Some(explicit_to), Some(next)) = (explicit_to_exclusive, next_valid_from) {
            if explicit_to > next {
                return Err(ScheduleError::OverlappingScheduleVersions {
                    first: valid_from.to_string(),
                    second: next.to_string(),
                });
            }
        }

        versions[i].effective_to = explicit_to_exclusive.or(next_valid_from);
    }
    Ok(())
}

fn version_for_date<'a>(
    versions: &'a [ScheduleVersion],
    country: &str,
    date: NaiveDate,
) -> Result<&'a ScheduleVersion, ScheduleError> {
    versions
        .iter()
        .find(|v| {
            let start = v.schedule.schedule.valid_from;
            let end = v.effective_to;
            start <= date && end.is_none_or(|e| date < e)
        })
        .ok_or_else(|| ScheduleError::NoScheduleForDate {
            country: country.to_string(),
            date: date.to_string(),
        })
}

fn dose_due_at(dose: &Dose, dob: NaiveDate) -> NaiveDate {
    dose.earliest_age.unwrap_or(dose.target_age).to_date(dob)
}

fn insert_effective_series(
    out: &mut Vec<Series>,
    index_by_id: &mut HashMap<String, usize>,
    mut series: Series,
    valid_from: NaiveDate,
    dob: NaiveDate,
) {
    if let Some(i) = index_by_id.get(&series.id).copied() {
        if compatible_series(&out[i], &series) {
            out[i].dose.append(&mut series.dose);
            out[i].dose.sort_by_key(|dose| dose.target_age.to_date(dob));
            return;
        }
    }

    if index_by_id.contains_key(&series.id) {
        series.id = format!("{}@{}", series.id, valid_from);
    }

    index_by_id.insert(series.id.clone(), out.len());
    out.push(series);
}

fn compatible_series(a: &Series, b: &Series) -> bool {
    a.display_name == b.display_name
        && a.product_class == b.product_class
        && a.antigens == b.antigens
        && a.eligibility.population == b.eligibility.population
        && a.eligibility.male_born_on_or_after == b.eligibility.male_born_on_or_after
}

fn validate_referential_integrity(schedule: &Schedule) -> Result<(), ScheduleError> {
    let known: std::collections::HashSet<&str> =
        schedule.antigen.iter().map(|a| a.id.as_str()).collect();
    for series in &schedule.series {
        for ant in &series.antigens {
            if !known.contains(ant.as_str()) {
                return Err(ScheduleError::UnknownAntigen {
                    series: series.id.clone(),
                    antigen: ant.clone(),
                });
            }
        }
    }
    Ok(())
}
