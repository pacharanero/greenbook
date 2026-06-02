use chrono::NaiveDate;
use greenbook::evaluate::{OverallStatus, SeriesCompletionStatus};
use greenbook::{evaluate, load_product_map, load_schedule, parse_fhir_bundle};
use std::path::Path;

#[test]
fn six_month_old_on_schedule_evaluates_correctly() {
    let schedule = load_schedule(Path::new("schedules/gb/2026-01-01.toml")).unwrap();
    let products = load_product_map(Path::new("products/gb-snomed-dm.toml")).unwrap();
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

    // Overall is Partial because some series are Complete and others not yet started.
    assert_eq!(status.overall, OverallStatus::PartiallyVaccinated);

    // Every recorded dose for the 6-in-1 series should be valid.
    let six_in_one = status
        .by_series
        .iter()
        .find(|s| s.series_id == "6in1-primary")
        .unwrap();
    assert_eq!(six_in_one.doses_recorded.len(), 3);
    assert!(six_in_one.doses_recorded.iter().all(|d| d.valid));
}
