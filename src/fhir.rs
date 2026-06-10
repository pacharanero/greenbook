use crate::error::FhirError;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct VaccinationRecord {
    pub patient_id: Option<String>,
    pub dob: NaiveDate,
    pub gender: Option<String>,
    pub immunisations: Vec<Immunisation>,
}

#[derive(Debug, Clone)]
pub struct Immunisation {
    pub date: NaiveDate,
    pub vaccine_code: String,
    pub vaccine_system: Option<String>,
    pub display: Option<String>,
    /// Dose number declared in `protocolApplied` (human-entered, may be wrong).
    pub dose_number: Option<u32>,
    /// SNOMED procedure code from the UKCore-VaccinationProcedure extension. This
    /// codes the *procedure* (which can name the dose), distinct from the dm+d
    /// product code in `vaccine_code`. Used as a duplicate signal and a
    /// dose-sequence cross-check.
    pub procedure_code: Option<String>,
    /// Display text for `procedure_code`, e.g. "Administration of second dose of
    /// ... vaccine (procedure)" - parsed for an explicit dose number.
    pub procedure_display: Option<String>,
}

/// The UKCore extension URL that carries the SNOMED vaccination-procedure code.
const VACCINATION_PROCEDURE_EXT: &str =
    "https://fhir.hl7.org.uk/StructureDefinition/Extension-UKCore-VaccinationProcedure";

#[derive(Debug, Deserialize)]
struct RawBundle {
    #[serde(default)]
    entry: Vec<RawEntry>,
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    resource: RawResource,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "resourceType")]
enum RawResource {
    Patient(RawPatient),
    Immunization(RawImmunization),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPatient {
    id: Option<String>,
    birth_date: NaiveDate,
    gender: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawImmunization {
    #[serde(default)]
    status: Option<String>,
    vaccine_code: RawCodeableConcept,
    occurrence_date_time: String,
    #[serde(default)]
    protocol_applied: Vec<RawProtocolApplied>,
    #[serde(default)]
    extension: Vec<RawExtension>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawExtension {
    url: String,
    #[serde(default)]
    value_codeable_concept: Option<RawCodeableConcept>,
}

#[derive(Debug, Deserialize)]
struct RawCodeableConcept {
    #[serde(default)]
    coding: Vec<RawCoding>,
}

#[derive(Debug, Deserialize)]
struct RawCoding {
    system: Option<String>,
    code: Option<String>,
    display: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProtocolApplied {
    dose_number_positive_int: Option<u32>,
}

pub fn parse_fhir_bundle(json: &str) -> Result<VaccinationRecord, FhirError> {
    let bundle: RawBundle = serde_json::from_str(json)?;

    let mut patient: Option<RawPatient> = None;
    let mut imms: Vec<RawImmunization> = Vec::new();

    for entry in bundle.entry {
        match entry.resource {
            RawResource::Patient(p) => patient = Some(p),
            RawResource::Immunization(i) => imms.push(i),
            RawResource::Other => {}
        }
    }

    let patient = patient.ok_or(FhirError::NoPatient)?;

    let mut immunisations = Vec::with_capacity(imms.len());
    for imm in imms {
        if let Some(status) = &imm.status {
            if status != "completed" {
                continue;
            }
        }
        // Pull the vaccination-procedure code (if present) before consuming
        // other fields. Its display can name the dose ("...second dose...").
        let (procedure_code, procedure_display) = imm
            .extension
            .iter()
            .find(|e| e.url == VACCINATION_PROCEDURE_EXT)
            .and_then(|e| e.value_codeable_concept.as_ref())
            .and_then(|cc| cc.coding.iter().find(|c| c.code.is_some()))
            .map(|c| (c.code.clone(), c.display.clone()))
            .unwrap_or((None, None));

        let coding = imm
            .vaccine_code
            .coding
            .into_iter()
            .find(|c| c.code.is_some())
            .ok_or(FhirError::MissingVaccineCode)?;
        immunisations.push(Immunisation {
            date: parse_fhir_datetime(&imm.occurrence_date_time)?,
            vaccine_code: coding.code.ok_or(FhirError::MissingVaccineCode)?,
            vaccine_system: coding.system,
            display: coding.display,
            dose_number: imm
                .protocol_applied
                .into_iter()
                .find_map(|p| p.dose_number_positive_int),
            procedure_code,
            procedure_display,
        });
    }

    immunisations.sort_by_key(|i| i.date);

    Ok(VaccinationRecord {
        patient_id: patient.id,
        dob: patient.birth_date,
        gender: patient.gender,
        immunisations,
    })
}

fn parse_fhir_datetime(s: &str) -> Result<NaiveDate, FhirError> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d);
    }
    if let Ok(dt) = DateTime::<FixedOffset>::parse_from_rfc3339(s) {
        return Ok(dt.date_naive());
    }
    if s.len() >= 10 {
        if let Ok(d) = NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d") {
            return Ok(d);
        }
    }
    Err(FhirError::BadDate(s.to_string()))
}
