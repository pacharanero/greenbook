use crate::age::AgeOffset;
use crate::error::ScheduleError;
use chrono::NaiveDate;
use serde::Deserialize;
use std::fs;
use std::path::Path;

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
