use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("empty age string")]
    Empty,

    #[error("expected a unit after number {0}")]
    MissingUnit(u32),

    #[error("unknown age unit `{0}` (expected days/weeks/months/years)")]
    UnknownUnit(String),

    #[error("invalid number `{0}`")]
    InvalidNumber(String),
}

#[derive(Debug, Error)]
pub enum ScheduleError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("age parse error: {0}")]
    Age(#[from] ParseError),

    #[error("series `{series}` references unknown antigen `{antigen}`")]
    UnknownAntigen { series: String, antigen: String },
}

#[derive(Debug, Error)]
pub enum FhirError {
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("bundle is missing a Patient resource")]
    NoPatient,

    #[error("Immunization is missing a SNOMED vaccineCode")]
    MissingVaccineCode,

    #[error("could not parse FHIR date/dateTime `{0}`")]
    BadDate(String),
}

#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error("schedule references unknown antigen `{0}` not in the antigen registry")]
    UnknownAntigen(String),
}
