use chrono::NaiveDate;
use greenbook::{
    evaluate, load_effective_schedule_for_date, load_product_map, load_schedule, parse_fhir_bundle,
    ScheduleError,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel)
}

fn temp_rules_dir(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "greenbook-{test_name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn pediacel_conforms_to_the_2006_schedule_but_not_the_2026_schedule() {
    let products = load_product_map(&repo_path("rules/product-map-uk-snomed-dm.toml")).unwrap();
    let bundle = r#"
{
  "resourceType": "Bundle",
  "type": "collection",
  "entry": [
    {
      "resource": {
        "resourceType": "Patient",
        "id": "patient-2006",
        "birthDate": "2006-11-01",
        "gender": "female"
      }
    },
    {
      "resource": {
        "resourceType": "Immunization",
        "status": "completed",
        "vaccineCode": {
          "coding": [{
            "system": "http://snomed.info/sct",
            "code": "9300601000001102",
            "display": "Pediacel (product)"
          }]
        },
        "occurrenceDateTime": "2007-01-01"
      }
    }
  ]
}
"#;
    let record = parse_fhir_bundle(bundle).unwrap();
    let evaluated_at = NaiveDate::from_ymd_opt(2007, 1, 15).unwrap();

    let schedule_2006 = load_schedule(&repo_path("rules/schedule-uk-2006-11-01.toml")).unwrap();
    let status_2006 = evaluate(&record, &schedule_2006, &products, evaluated_at).unwrap();
    let five_in_one = status_2006
        .by_series
        .iter()
        .find(|s| s.series_id == "5in1-primary")
        .unwrap();
    assert_eq!(five_in_one.doses_valid, 1);
    assert!(five_in_one.doses_recorded[0].within_schedule);
    assert!(status_2006.unmatched_doses.is_empty());

    let schedule_2026 = load_schedule(&repo_path("rules/schedule-uk-2026-01-01.toml")).unwrap();
    let status_2026 = evaluate(&record, &schedule_2026, &products, evaluated_at).unwrap();
    assert_eq!(status_2026.unmatched_doses.len(), 1);
    assert_eq!(
        status_2026.unmatched_doses[0].reason,
        "product class \"5-in-1\" has no series in this schedule version"
    );
}

fn write_schedule(dir: &Path, date: &str, valid_to: Option<&str>, product_class: &str) {
    let valid_to_line = valid_to
        .map(|d| format!("valid_to = \"{d}\"\n"))
        .unwrap_or_default();
    let schedule = format!(
        r#"
[jurisdiction]
country = "UK"
schedule_authority = "Test"
product_coding_system = "test"
language = "en-GB"

[schedule]
valid_from = "{date}"
{valid_to_line}source_document = "Synthetic {date}"
source_url = "https://example.test/{date}"

[[series]]
id = "primary"
display_name = "Primary"
description = "Synthetic primary course."
product_class = "{product_class}"
antigens = ["example"]

[series.eligibility]
population = "all"

[[series.dose]]
number = 1
target_age = "8 weeks"
earliest_age = "8 weeks"

[[series.dose]]
number = 2
target_age = "12 weeks"
earliest_age = "12 weeks"
min_interval_from_previous = "4 weeks"

[[series.dose]]
number = 3
target_age = "16 weeks"
earliest_age = "16 weeks"
min_interval_from_previous = "4 weeks"

[[antigen]]
id = "example"
display_name = "Example"
snomed_concept = "1"
"#
    );
    fs::write(dir.join(format!("schedule-uk-{date}.toml")), schedule).unwrap();
}

#[test]
fn effective_schedule_selects_dose_slots_by_due_date_version() {
    let dir = temp_rules_dir("due-date-version");
    write_schedule(&dir, "2020-01-01", Some("2020-12-31"), "5-in-1");
    write_schedule(&dir, "2021-01-01", None, "6-in-1");

    let dob = NaiveDate::from_ymd_opt(2020, 10, 1).unwrap();
    let evaluated_at = NaiveDate::from_ymd_opt(2021, 3, 1).unwrap();
    let historical = load_effective_schedule_for_date(&dir, "UK", dob, evaluated_at).unwrap();

    assert_eq!(historical.selection.versions.len(), 2);
    assert_eq!(historical.schedule.series.len(), 2);

    let old_series = historical
        .schedule
        .series
        .iter()
        .find(|s| s.product_class == "5-in-1")
        .unwrap();
    assert_eq!(old_series.id, "primary");
    assert_eq!(
        old_series.dose.iter().map(|d| d.number).collect::<Vec<_>>(),
        vec![1, 2]
    );

    let new_series = historical
        .schedule
        .series
        .iter()
        .find(|s| s.product_class == "6-in-1")
        .unwrap();
    assert_eq!(new_series.id, "primary@2021-01-01");
    assert_eq!(
        new_series.dose.iter().map(|d| d.number).collect::<Vec<_>>(),
        vec![3]
    );
}

#[test]
fn effective_schedule_reports_gap_when_no_version_covers_due_date() {
    let dir = temp_rules_dir("gap");
    write_schedule(&dir, "2020-01-01", Some("2020-11-30"), "5-in-1");
    write_schedule(&dir, "2021-01-01", None, "6-in-1");

    let dob = NaiveDate::from_ymd_opt(2020, 10, 1).unwrap();
    let evaluated_at = NaiveDate::from_ymd_opt(2021, 3, 1).unwrap();
    let err = load_effective_schedule_for_date(&dir, "UK", dob, evaluated_at).unwrap_err();

    match err {
        ScheduleError::NoScheduleForDate { date, .. } => {
            assert_eq!(date, "2020-12-24");
        }
        other => panic!("unexpected error: {other}"),
    }
}
