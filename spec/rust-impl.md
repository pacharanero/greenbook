# Rust Implementation

This document describes the reference Rust implementation: the processing pipeline, public types, and CLI surface. The crate lives in [`rust/`](../rust/) - see [rust/README.md](../rust/README.md) for build, test, and CLI usage; the JavaScript implementation is in [`js/`](../js/). Both are validated against the shared [`conformance/`](../conformance/) suite.

---

## Architecture

### Input

A [FHIR R4](https://hl7.org/fhir/R4/) Bundle containing:

- One [`Patient`](https://hl7.org/fhir/R4/patient.html) resource (for date of birth)
- One or more [`Immunization`](https://hl7.org/fhir/R4/immunization.html) resources (one per vaccination event)

NHS Digital uses FHIR R4 with [UK Core](https://simplifier.net/hl7fhirukcorer4) profiles. The `vaccineCode` field in UK records uses [SNOMED CT](https://www.snomed.org/) (UK drug extension), e.g. `39114911000001105` = "Infanrix Hexa vaccine (product)".

Minimum required fields per `Immunization` resource:

- `status` = `"completed"`
- `vaccineCode.coding[].system` and `.code` (SNOMED CT)
- `occurrenceDateTime`
- `patient.reference`

### Processing pipeline

```
Bundle (Patient + [Immunization])
  |
  v
[1] parse_record()
    - extract DOB from Patient resource
    - extract (vaccine_code, date) pairs from Immunization resources
    - map vaccine_code -> antigen(s) via product mapping table

  |
  v
[2] load_schedule()
    - load schedule TOML for the relevant version
    - for v1: load current schedule only
    - for historical: build an effective schedule from the version in force
      when each dose slot first became due for the patient

  |
  v
[3] evaluate_per_series()
    - for each series in the schedule:
      - check eligibility (population, sex, birth cohort)
      - determine expected doses, marking each due / not-yet-due
      - find matching received doses (by product class - see conformance-vs-coverage)
      - validate each dose: age at administration, interval from prior dose;
        flag out-of-schedule doses (too early / too late)
      - classify: COMPLETE | PARTIAL | NONE | NOT_APPLICABLE

  |
  v
[4] aggregate()
    - headline status: UP_TO_DATE_FOR_AGE | BEHIND_FOR_AGE |
                       UNVACCINATED | UNKNOWN
    - fully_vaccinated flag (strict: every applicable series COMPLETE)
    - per-series breakdown
    - per-antigen breakdown (coverage; deferred)

  |
  v
Output: VaccinationStatus (structured JSON or human-readable report)
```

---

## Rust Types

### Core types

```rust
/// A duration offset from date of birth, parsed from strings like
/// "8 weeks", "12 months", "3 years 4 months".
/// Stored internally in days for comparison.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AgeOffset {
    pub days: u32,
}

impl AgeOffset {
    pub fn from_str(s: &str) -> Result<Self, ParseError> { ... }
    pub fn to_date(&self, dob: NaiveDate) -> NaiveDate { ... }
}

/// A duration interval, e.g. "4 weeks", "6 months".
#[derive(Debug, Clone)]
pub struct Interval {
    pub days: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Schedule {
    pub jurisdiction: Jurisdiction,
    pub schedule: ScheduleMeta,
    pub series: Vec<Series>,
    pub antigen: Vec<Antigen>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Jurisdiction {
    pub country: String,           // ISO 3166 code; "UK" (exceptionally-reserved) rather than the primary alpha-2 "GB"
    pub schedule_authority: String,
    pub product_coding_system: String,
    pub language: String,          // BCP 47
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleMeta {
    pub valid_from: NaiveDate,
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
    pub product_class: String,     // conformance: doses of this class match (see conformance-vs-coverage)
    pub antigens: Vec<String>,     // references Antigen.id; coverage view only
    pub eligibility: Eligibility,
    pub dose: Vec<Dose>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Eligibility {
    pub population: String,        // "all" | "female" | "male"
    pub male_born_on_or_after: Option<NaiveDate>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dose {
    pub number: u32,
    pub target_age: AgeOffset,
    pub earliest_age: Option<AgeOffset>,
    pub latest_age: Option<AgeOffset>,
    pub min_interval_from_previous: Option<Interval>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Antigen {
    pub id: String,
    pub display_name: String,
    pub snomed_concept: String,
}
```

### Evaluation output types

```rust
#[derive(Debug, Clone, Serialize)]
pub struct VaccinationStatus {
    pub status: OverallStatus,      // headline, age-relative determination
    pub fully_vaccinated: bool,     // strict: every applicable series is Complete (all ages)
    pub evaluated_at: NaiveDate,    // date of evaluation (today, or a specified date)
    pub schedule_version: NaiveDate,
    pub by_series: HashMap<String, SeriesStatus>,
    pub unmatched_doses: Vec<UnmatchedDose>,         // matched no series at all
    pub duplicate_doses: Vec<DuplicateDose>,         // echoes (same procedure code)
    pub by_antigen: HashMap<String, AntigenStatus>,  // coverage view; deferred
}

/// The headline answer to "is this patient correctly vaccinated for their age?"
/// Distinct from the strict `fully_vaccinated` flag on VaccinationStatus, which
/// asks the age-independent "have they had every dose the schedule ever defines?".
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum OverallStatus {
    UpToDateForAge,   // every dose due by evaluated_at has been received and is valid
    BehindForAge,     // a dose already due is missing or only met by an out-of-schedule dose
    Unvaccinated,     // no valid doses recorded at all
    Unknown,          // DOB missing, or cannot distinguish "none given" from "no data"
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesStatus {
    pub series_id: String,
    pub display_name: String,
    pub status: SeriesCompletionStatus,
    pub doses_expected: u32,
    pub doses_due: u32,             // of the expected doses, how many are due by evaluated_at
    pub doses_valid: u32,
    pub up_to_date_for_age: bool,   // every dose due so far has a valid recorded dose
    pub doses_recorded: Vec<RecordedDose>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SeriesCompletionStatus {
    Complete,
    Partial,
    None,
    NotApplicable,  // patient not eligible for this series
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordedDose {
    pub date: NaiveDate,
    pub age_at_dose: AgeOffset,
    pub vaccine_code: String,
    pub assigned_dose_number: Option<u32>,  // which dose slot it filled, by date order
    pub within_schedule: bool,        // false => given outside the standard schedule
    pub schedule_notes: Vec<String>,  // e.g. "given before earliest_age (outside standard schedule)"
    pub flags: Vec<String>,           // soft cross-check warnings, e.g. dose number disagrees with date order
}

/// A recorded dose dropped as a likely duplicate "echo" of an earlier dose with
/// the same procedure code (see standard.md §"Duplicate doses").
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateDose {
    pub date: NaiveDate,
    pub vaccine_code: String,
    pub display: Option<String>,
    pub procedure_code: Option<String>,
    pub duplicate_of: NaiveDate,      // the kept dose this one echoes
}
```

### Public library API

```rust
/// Load a schedule from a TOML file.
pub fn load_schedule(path: &Path) -> Result<Schedule, ScheduleError>;

/// Load every schedule version for a jurisdiction from rules/schedule-<country>-*.toml.
pub fn load_schedule_versions(
    rules_dir: &Path,
    country: &str,
) -> Result<Vec<ScheduleVersion>, ScheduleError>;

/// Build the effective historical schedule for a patient/evaluation date by
/// selecting each dose slot from the schedule version in force when that dose
/// first became due.
pub fn load_effective_schedule_for_date(
    rules_dir: &Path,
    country: &str,
    dob: NaiveDate,
    evaluated_at: NaiveDate,
) -> Result<HistoricalSchedule, ScheduleError>;

/// Parse a FHIR R4 Bundle JSON into a VaccinationRecord.
pub fn parse_fhir_bundle(json: &str) -> Result<VaccinationRecord, FhirError>;

/// Evaluate a vaccination record against a schedule.
pub fn evaluate(
    record: &VaccinationRecord,
    schedule: &Schedule,
    product_map: &ProductMap,
) -> Result<VaccinationStatus, EvaluationError>;
```

---

## CLI

### Commands

```
greenbook versions <rules-dir> [--country <code>]
```

Lists all available schedule versions for a jurisdiction, with valid_from date, source document name, and change summary.

```
greenbook evaluate <schedule-file> <product-map> <fhir-bundle> [--evaluated-at YYYY-MM-DD] [--format report|json|status]
```

Evaluates a patient's vaccination history against one explicit schedule file. This is still useful for fixtures, conformance, and testing a proposed schedule file in isolation.

```
greenbook evaluate-auto <rules-dir> <product-map> <fhir-bundle> [--country UK] [--evaluated-at YYYY-MM-DD] [--format report|json|status] [--verbose]
```

Builds the effective historical schedule from every schedule file in `rules-dir`, selecting dose slots by the version in force when each slot first became due. `--verbose` includes the schedule-selection rationale in report output.

```
greenbook render <schedule-file> [--format table|markdown|html]
```

Renders the schedule as an age-centric table (as seen in Green Book Chapter 11). Default format is `table` (plain text, box-drawing characters).

```
greenbook diff <schedule-file-a> <schedule-file-b> [--format table|json]
```

Compares two schedule versions, showing what changed between them. Useful for reviewing proposed schedule changes in pull requests.

```
greenbook validate <schedule-file>
```

Validates a schedule TOML file for structural correctness, referential integrity (all antigen IDs in series blocks exist in the antigen registry), and logical consistency (dose numbers sequential, min_interval present where dose > 1, etc.).

### Example rendered output

```
NHS Routine Childhood Immunisation Schedule (UK, effective 2026-01-01)

Age               | Vaccines
------------------+------------------------------------------------------------------
8 weeks           | 6-in-1 (dose 1), Rotavirus (dose 1), MenB (dose 1)
12 weeks          | 6-in-1 (dose 2), Rotavirus (dose 2), PCV (dose 1)
16 weeks          | 6-in-1 (dose 3), MenB (dose 2)
12 months         | Hib/MenC booster, MMR (dose 1), MenB (dose 3), PCV (dose 2)
3 years 4 months  | MMR (dose 2), 4-in-1 pre-school booster
12-13 years       | HPV (2 doses, girls and boys)
14 years          | Td/IPV booster (3-in-1 teenage booster)
```

---

## Crate Structure

```
greenbook/
  spec/                  # specification documents (this file, standard.md, ...)
  rules/                 # canonical sources (top-level, language-neutral)
    schedule-uk-2026-01-01.toml
    product-map-uk-snomed-dm.toml
  conformance/           # shared test harness
    cases.json           # case manifest
    fixtures/*.json      # FHIR bundles
    expected/*.json      # golden outputs (Rust-generated; all impls test against these)
  rust/                  # this (reference) implementation
    Cargo.toml, README.md
    src/
      lib.rs             # public API
      evaluate.rs        # evaluation engine
      fhir.rs            # FHIR Bundle parser (Patient + Immunization)
      schedule.rs        # Schedule, Series, Dose, Antigen types + TOML deserialisation
      products.rs        # ProductMap - vaccine code to class/antigen mapping
      age.rs             # AgeOffset parsing
      error.rs           # error types
    src/bin/
      greenbook.rs       # CLI entry point
      conformance.rs     # conformance golden generator
    tests/
      evaluate.rs        # integration tests
      conformance.rs     # shared-conformance check (cargo test)
  js/                    # the JavaScript implementation (greenbook.js + test/)
  docs/                  # presentation + demo
```

---

## Build and Tooling Conventions

- [Rust edition 2021](https://doc.rust-lang.org/edition-guide/rust-2021/index.html)
- [`serde`](https://serde.rs/) + [`toml`](https://crates.io/crates/toml) for schedule deserialisation
- [`serde`](https://serde.rs/) + [`serde_json`](https://crates.io/crates/serde_json) for FHIR bundle parsing
- [`clap`](https://crates.io/crates/clap) for CLI argument parsing
- [`chrono`](https://crates.io/crates/chrono) or [`time`](https://crates.io/crates/time) for date arithmetic
- [`thiserror`](https://crates.io/crates/thiserror) for error types
- CI: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- Licence: [AGPL-3.0](https://www.gnu.org/licenses/agpl-3.0.html) (consistent with GitEHR and other [RCPCH](https://www.rcpch.ac.uk/) clinical projects)
