pub mod age;
pub mod error;
pub mod evaluate;
pub mod fhir;
pub mod products;
pub mod schedule;

pub use age::AgeOffset;
pub use error::{EvaluationError, FhirError, ParseError, ScheduleError};
pub use evaluate::{
    evaluate, OverallStatus, RecordedDose, SeriesCompletionStatus, SeriesStatus, VaccinationStatus,
};
pub use fhir::{parse_fhir_bundle, Immunisation, VaccinationRecord};
pub use products::{load_product_map, Product, ProductMap};
pub use schedule::{load_schedule, Antigen, Dose, Eligibility, Jurisdiction, Schedule, Series};
