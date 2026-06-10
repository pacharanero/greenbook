use chrono::NaiveDate;
use greenbook::evaluate::{OverallStatus, SeriesCompletionStatus};
use greenbook::{evaluate, load_product_map, load_schedule, parse_fhir_bundle};
use std::path::Path;

#[test]
fn six_month_old_on_schedule_evaluates_correctly() {
    let schedule = load_schedule(Path::new("schedules/uk-2026-01-01.toml")).unwrap();
    let products = load_product_map(Path::new("products/uk-snomed-dm.toml")).unwrap();
    let bundle = std::fs::read_to_string("tests/fixtures/six-month-fully-vaccinated.json").unwrap();
    let record = parse_fhir_bundle(&bundle).unwrap();

    let evaluated_at = NaiveDate::from_ymd_opt(2026, 4, 29).unwrap();
    let status = evaluate(&record, &schedule, &products, evaluated_at).unwrap();

    let series_status = |id: &str| -> SeriesCompletionStatus {
        status
            .by_series
            .iter()
            .find(|s| s.series_id == id)
            .unwrap_or_else(|| panic!("series {} not in result", id))
            .status
    };

    assert_eq!(
        series_status("6in1-primary"),
        SeriesCompletionStatus::Complete
    );
    assert_eq!(
        series_status("rotavirus-primary"),
        SeriesCompletionStatus::Complete
    );

    assert_eq!(
        series_status("menb-primary"),
        SeriesCompletionStatus::Partial
    );
    assert_eq!(
        series_status("pcv-primary"),
        SeriesCompletionStatus::Partial
    );

    assert_eq!(
        series_status("hib-menc-booster"),
        SeriesCompletionStatus::None
    );
    assert_eq!(series_status("mmr-primary"), SeriesCompletionStatus::None);
    assert_eq!(series_status("mmr-second"), SeriesCompletionStatus::None);
    assert_eq!(series_status("hpv-primary"), SeriesCompletionStatus::None);
    assert_eq!(
        series_status("tdap-ipv-booster"),
        SeriesCompletionStatus::None
    );

    // Headline status: the infant has every dose due so far, so they are
    // up-to-date for age - even though later doses (MMR, HPV, ...) are not given.
    assert_eq!(status.status, OverallStatus::UpToDateForAge);
    // But not "fully vaccinated" in the strict sense: not every series complete.
    assert!(!status.fully_vaccinated);

    // Every recorded dose for the 6-in-1 series should be within the schedule.
    let six_in_one = status
        .by_series
        .iter()
        .find(|s| s.series_id == "6in1-primary")
        .unwrap();
    assert_eq!(six_in_one.doses_recorded.len(), 3);
    assert!(six_in_one.doses_recorded.iter().all(|d| d.within_schedule));

    // Not-yet-due series are up-to-date for age (nothing due means no gap).
    let hpv = status
        .by_series
        .iter()
        .find(|s| s.series_id == "hpv-primary")
        .unwrap();
    assert_eq!(hpv.doses_due, 0);
    assert!(hpv.up_to_date_for_age);

    // This clean fixture should leave no unmatched doses.
    assert!(status.unmatched_doses.is_empty());
}

/// Shared helper: load the bundled UK schedule + product map and evaluate a
/// fixture at a fixed date, so every test is deterministic.
fn evaluate_fixture(fixture: &str, evaluated_at: NaiveDate) -> greenbook::VaccinationStatus {
    let schedule = load_schedule(Path::new("schedules/uk-2026-01-01.toml")).unwrap();
    let products = load_product_map(Path::new("products/uk-snomed-dm.toml")).unwrap();
    let bundle = std::fs::read_to_string(format!("tests/fixtures/{fixture}")).unwrap();
    let record = parse_fhir_bundle(&bundle).unwrap();
    evaluate(&record, &schedule, &products, evaluated_at).unwrap()
}

#[test]
fn toddler_missing_twelve_month_doses_is_behind_for_age() {
    let status = evaluate_fixture(
        "behind-for-age-toddler.json",
        NaiveDate::from_ymd_opt(2026, 4, 29).unwrap(),
    );

    // Doses that are due but missing make the patient behind, not "partial".
    assert_eq!(status.status, OverallStatus::BehindForAge);

    // The 12-month doses are due by 18 months but were not given.
    let series = |id: &str| status.by_series.iter().find(|s| s.series_id == id).unwrap();
    assert!(!series("hib-menc-booster").up_to_date_for_age);
    assert!(!series("mmr-primary").up_to_date_for_age);
    // The primary infant course was completed on time.
    assert!(series("6in1-primary").up_to_date_for_age);
}

#[test]
fn out_of_schedule_doses_are_flagged_and_do_not_count() {
    let status = evaluate_fixture(
        "out-of-schedule-doses.json",
        NaiveDate::from_ymd_opt(2026, 4, 29).unwrap(),
    );

    // The 6-in-1 second dose was given a week after the first: interval too
    // short, so it is outside the standard schedule and does not count.
    let six_in_one = status
        .by_series
        .iter()
        .find(|s| s.series_id == "6in1-primary")
        .unwrap();
    assert_eq!(six_in_one.doses_valid, 1);
    let dose2 = &six_in_one.doses_recorded[1];
    assert!(!dose2.within_schedule);
    assert!(!dose2.schedule_notes.is_empty());

    // The rotavirus dose was given after its hard cutoff.
    let rotavirus = status
        .by_series
        .iter()
        .find(|s| s.series_id == "rotavirus-primary")
        .unwrap();
    assert_eq!(rotavirus.doses_valid, 0);
    assert!(rotavirus.doses_recorded.iter().all(|d| !d.within_schedule));
}

#[test]
fn unknown_and_superseded_products_are_reported_as_unmatched() {
    let status = evaluate_fixture(
        "unmatched-doses.json",
        NaiveDate::from_ymd_opt(2026, 4, 29).unwrap(),
    );

    // A Pediacel (5-in-1) dose and an unknown code both match no series.
    assert_eq!(status.unmatched_doses.len(), 2);
    // The valid 6-in-1 dose is still matched and counted normally.
    let six_in_one = status
        .by_series
        .iter()
        .find(|s| s.series_id == "6in1-primary")
        .unwrap();
    assert_eq!(six_in_one.doses_valid, 1);
}

#[test]
fn both_mmr_doses_allocate_across_the_two_mmr_series() {
    let status = evaluate_fixture(
        "mmr-both-doses.json",
        NaiveDate::from_ymd_opt(2026, 4, 29).unwrap(),
    );
    let series = |id: &str| status.by_series.iter().find(|s| s.series_id == id).unwrap();

    // The shared MMR class is one programme: dose 1 -> mmr-primary, dose 2 ->
    // mmr-second. Both Complete, each with exactly one in-schedule dose, and -
    // crucially - no spurious "extra" / "too early" flags from cross-matching.
    let primary = series("mmr-primary");
    let second = series("mmr-second");
    assert_eq!(primary.status, SeriesCompletionStatus::Complete);
    assert_eq!(second.status, SeriesCompletionStatus::Complete);
    assert_eq!(primary.doses_recorded.len(), 1);
    assert_eq!(second.doses_recorded.len(), 1);
    assert!(primary
        .doses_recorded
        .iter()
        .all(|d| d.within_schedule && d.flags.is_empty()));
    assert!(second
        .doses_recorded
        .iter()
        .all(|d| d.within_schedule && d.flags.is_empty()));
    assert!(status.duplicate_doses.is_empty());
}

#[test]
fn echoed_dose_with_same_procedure_code_is_a_duplicate() {
    let status = evaluate_fixture(
        "duplicate-echo.json",
        NaiveDate::from_ymd_opt(2026, 4, 29).unwrap(),
    );

    // The two 6-in-1 records share a procedure code: the later is a duplicate,
    // not a second dose. So the series counts one valid dose, and the echo is
    // reported separately.
    assert_eq!(status.duplicate_doses.len(), 1);
    let six_in_one = status
        .by_series
        .iter()
        .find(|s| s.series_id == "6in1-primary")
        .unwrap();
    assert_eq!(six_in_one.doses_valid, 1);
    assert_eq!(six_in_one.doses_recorded.len(), 1);
}

#[test]
fn mis_keyed_dose_number_is_flagged_not_trusted() {
    let status = evaluate_fixture(
        "dose-number-mismatch.json",
        NaiveDate::from_ymd_opt(2026, 4, 29).unwrap(),
    );

    // The dose is the first MMR by date but recorded as dose 2. It is allocated
    // to dose 1 (date wins) and flagged - valid, but flagged for review.
    let primary = status
        .by_series
        .iter()
        .find(|s| s.series_id == "mmr-primary")
        .unwrap();
    assert_eq!(primary.doses_recorded.len(), 1);
    let dose = &primary.doses_recorded[0];
    assert!(dose.within_schedule);
    assert_eq!(dose.assigned_dose_number, Some(1));
    assert!(!dose.flags.is_empty());
}
