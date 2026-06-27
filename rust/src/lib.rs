pub mod age;
pub mod error;
pub mod evaluate;
pub mod fhir;
pub mod products;
pub mod schedule;

pub use age::AgeOffset;
pub use error::{EvaluationError, FhirError, ParseError, ScheduleError};
pub use evaluate::{
    evaluate, DuplicateDose, OverallStatus, RecordedDose, SeriesCompletionStatus, SeriesStatus,
    UnmatchedDose, VaccinationStatus,
};
pub use fhir::{parse_fhir_bundle, Immunisation, VaccinationRecord};
pub use products::{load_product_map, Product, ProductMap};
pub use schedule::{
    effective_schedule_for_versions, load_effective_schedule_for_date, load_schedule,
    load_schedule_versions, Antigen, Dose, Eligibility, HistoricalSchedule, Jurisdiction, Schedule,
    ScheduleSelection, ScheduleVersion, ScheduleVersionSummary, Series,
};
