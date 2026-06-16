# The Standard

This document defines the data formats and evaluation semantics. It is implementation-neutral: any language that can parse TOML and JSON can implement a conforming evaluator.

---

## File Format: `schedule.toml`

### Design principles

- [TOML](https://toml.io/) (Tom's Obvious Minimal Language)
- Human-readable without training; editable by clinical informaticians, ideally with GUI tooling support
- Strongly typed (TOML's type system prevents YAML-style ambiguities)
- Schedule is **series-centric** for authoring; the CLI `render` command produces the **age-centric** table for publication
- Each schedule version is a separate dated file
- Global by design with a `jurisdiction` block to determine where it's applicable.
- Files are tracked in git, giving a full audit trail of who changed what and when

### Directory structure

Both kinds of canonical source - schedule versions and product mappings - live together in a single top-level `rules/` directory. A filename prefix carries the kind, so the two are unmistakable and each globs cleanly (`schedule-*.toml`, `product-map-*.toml`):

```
rules/
  schedule-uk-2026-01-01.toml      # current schedule
  schedule-uk-2020-01-01.toml
  schedule-uk-2015-09-01.toml
  ...
  product-map-uk-snomed-dm.toml    # UK SNOMED drug extension product mapping
```

After the `schedule-` prefix, a schedule file is named `<jurisdiction>-<valid_from>` so its effective date is inferable from the name without parsing the file. A product map is named `product-map-<jurisdiction>-<coding-system>`.

The format is global by design (the `jurisdiction` block carries the country and authority), so adding a second jurisdiction needs no format change - the jurisdiction code is already in the filename, so the flat layout absorbs new countries directly:

```
rules/
  schedule-uk-2026-01-01.toml
  schedule-us-2026-01-01.toml
  product-map-uk-snomed-dm.toml
  product-map-us-cvx.toml
```

Should one jurisdiction's history grow large enough to warrant it, files may later be grouped into per-kind or per-country subdirectories (`rules/uk/schedule-2026-01-01.toml`) without a format change. No non-UK data exists yet.

The jurisdiction code is `UK`. ISO 3166-1 alpha-2 assigns the United Kingdom the code `GB` ("United Kingdom of Great Britain and Northern Ireland"), which does include Northern Ireland - but because the label "Great Britain" reads as excluding it, this project uses the ISO 3166 exceptionally-reserved code `UK` to make the UK-wide scope of the Green Book unambiguous. (The BCP 47 language tag stays `en-GB`; there is no `en-UK`.)

### Full annotated example

```toml
# =============================================================================
# NHS Routine Childhood Immunisation Schedule
# =============================================================================
# This file is the primary computable representation of the schedule.
# The PDF chapter is generated from this file - not the other way around.
#
# To propose a change: copy this file with the new valid_from date as the
# filename, edit it, and open a pull request.
# =============================================================================

[jurisdiction]
country = "UK"                          # ISO 3166 exceptionally-reserved code; used instead of the primary alpha-2 "GB" so the UK-wide scope (incl. Northern Ireland) is unambiguous
country_name = "United Kingdom"
schedule_authority = "UKHSA"
schedule_authority_url = "https://www.gov.uk/government/organisations/uk-health-security-agency"
product_coding_system = "snomed-uk-dm" # snomed-uk-dm | cvx | amt | snomed-int
language = "en-GB"                     # BCP 47 (https://www.rfc-editor.org/info/bcp47)

[schedule]
valid_from = "2026-01-01"
supersedes = "2020-01-01"
source_document = "Green Book Chapter 11, updated 30 March 2026"
source_url = "https://assets.publishing.service.gov.uk/media/..."
change_summary = """
  Added 18-month vaccination appointment.
  See CHANGELOG.md for full details.
"""

# ---------- SERIES ----------
# One [[series]] block per vaccination programme.
# antigens references IDs defined in the [[antigen]] registry below.

[[series]]
id = "6in1-primary"
display_name = "6-in-1"
description = """
  Primary immunisation against diphtheria, tetanus, pertussis (whooping cough),
  polio, Hib (Haemophilus influenzae type b) and hepatitis B.
"""
product_class = "6-in-1"   # conformance: doses of this class match this series
antigens = ["diphtheria", "tetanus", "pertussis", "polio", "hib", "hepatitis-b"]  # coverage only

[series.eligibility]
population = "all"

[[series.dose]]
number = 1
target_age = "8 weeks"
earliest_age = "8 weeks"

[[series.dose]]
number = 2
target_age = "12 weeks"
earliest_age = "10 weeks"
min_interval_from_previous = "4 weeks"

[[series.dose]]
number = 3
target_age = "16 weeks"
earliest_age = "14 weeks"
min_interval_from_previous = "4 weeks"


[[series]]
id = "rotavirus-primary"
display_name = "Rotavirus"
description = "Primary immunisation against rotavirus gastroenteritis."
antigens = ["rotavirus"]

[series.eligibility]
population = "all"
notes = "First dose must be given before 15 weeks. Course must be completed by 24 weeks."

[[series.dose]]
number = 1
target_age = "8 weeks"
earliest_age = "6 weeks"
latest_age = "14 weeks 6 days"   # hard cutoff - do not give after 15 weeks

[[series.dose]]
number = 2
target_age = "12 weeks"
min_interval_from_previous = "4 weeks"
latest_age = "23 weeks 6 days"   # hard cutoff - course must complete by 24 weeks


[[series]]
id = "menb-primary"
display_name = "MenB"
description = "Immunisation against meningococcal group B disease."
antigens = ["meningococcal-b"]

[series.eligibility]
population = "all"

[[series.dose]]
number = 1
target_age = "8 weeks"
earliest_age = "8 weeks"

[[series.dose]]
number = 2
target_age = "16 weeks"
earliest_age = "14 weeks"
min_interval_from_previous = "4 weeks"

[[series.dose]]
number = 3
target_age = "12 months"
earliest_age = "11 months"
min_interval_from_previous = "6 months"


[[series]]
id = "pcv-primary"
display_name = "PCV (pneumococcal)"
description = "Immunisation against pneumococcal disease."
antigens = ["pneumococcal"]

[series.eligibility]
population = "all"

[[series.dose]]
number = 1
target_age = "12 weeks"
earliest_age = "8 weeks"

[[series.dose]]
number = 2
target_age = "12 months"
earliest_age = "11 months"
min_interval_from_previous = "8 weeks"


[[series]]
id = "hib-menc-booster"
display_name = "Hib/MenC booster"
description = "Booster providing additional protection against Hib and meningococcal group C."
antigens = ["hib", "meningococcal-c"]

[series.eligibility]
population = "all"

[[series.dose]]
number = 1
target_age = "12 months"
earliest_age = "11 months"


[[series]]
id = "mmr-primary"
display_name = "MMR (first dose)"
description = "First dose of measles, mumps and rubella vaccine."
antigens = ["measles", "mumps", "rubella"]

[series.eligibility]
population = "all"

[[series.dose]]
number = 1
target_age = "12 months"
earliest_age = "11 months"


[[series]]
id = "mmr-second"
display_name = "MMR (second dose)"
description = "Second dose of measles, mumps and rubella vaccine."
antigens = ["measles", "mumps", "rubella"]

[series.eligibility]
population = "all"

[[series.dose]]
number = 2
target_age = "3 years 4 months"
earliest_age = "3 years"
min_interval_from_previous = "1 month"


[[series]]
id = "hpv-primary"
display_name = "HPV"
description = """
  Immunisation against human papillomavirus types causing cervical cancer
  and other HPV-related cancers.
"""
antigens = ["hpv"]

[series.eligibility]
population = "all"
notes = """
  All females up to 25th birthday.
  Males born on or after 2006-09-01 up to 25th birthday.
"""
male_born_on_or_after = "2006-09-01"

[[series.dose]]
number = 1
target_age = "12 years"     # delivered in school year 8
earliest_age = "11 years 6 months"

[[series.dose]]
number = 2
target_age = "13 years"
min_interval_from_previous = "6 months"


[[series]]
id = "tdap-ipv-booster"
display_name = "Td/IPV booster (3-in-1 teenage booster)"
description = "Booster against tetanus, diphtheria and polio."
antigens = ["tetanus", "diphtheria", "polio"]

[series.eligibility]
population = "all"

[[series.dose]]
number = 1
target_age = "14 years"     # school year 9
earliest_age = "13 years 6 months"


# ---------- ANTIGEN REGISTRY ----------
# Short IDs used in series blocks map to SNOMED CT concepts here.
# SNOMED CT concept codes are international and stable.
# These IDs are intended to be stable across schedule versions and jurisdictions.

[[antigen]]
id = "diphtheria"
display_name = "Diphtheria"
snomed_concept = "397428000"
snomed_description = "Diphtheria (disorder)"

[[antigen]]
id = "tetanus"
display_name = "Tetanus"
snomed_concept = "76902006"
snomed_description = "Tetanus (disorder)"

[[antigen]]
id = "pertussis"
display_name = "Pertussis (whooping cough)"
snomed_concept = "27836007"
snomed_description = "Pertussis (disorder)"

[[antigen]]
id = "polio"
display_name = "Poliomyelitis"
snomed_concept = "398102009"
snomed_description = "Acute poliomyelitis (disorder)"

[[antigen]]
id = "hib"
display_name = "Haemophilus influenzae type b (Hib)"
snomed_concept = "433692003"
snomed_description = "Haemophilus influenzae type b infection (disorder)"

[[antigen]]
id = "hepatitis-b"
display_name = "Hepatitis B"
snomed_concept = "66071002"
snomed_description = "Hepatitis B (disorder)"

[[antigen]]
id = "rotavirus"
display_name = "Rotavirus"
snomed_concept = "18624000"
snomed_description = "Disease due to Rotavirus (disorder)"

[[antigen]]
id = "meningococcal-b"
display_name = "Meningococcal group B"
snomed_concept = "860805006"
snomed_description = "Infection caused by Neisseria meningitidis serogroup B (disorder)"

[[antigen]]
id = "meningococcal-c"
display_name = "Meningococcal group C"
snomed_concept = "860806007"
snomed_description = "Infection caused by Neisseria meningitidis serogroup C (disorder)"

[[antigen]]
id = "pneumococcal"
display_name = "Pneumococcal disease"
snomed_concept = "16814004"
snomed_description = "Pneumococcal infectious disease (disorder)"

[[antigen]]
id = "measles"
display_name = "Measles"
snomed_concept = "14189004"
snomed_description = "Measles (disorder)"

[[antigen]]
id = "mumps"
display_name = "Mumps"
snomed_concept = "36989005"
snomed_description = "Mumps (disorder)"

[[antigen]]
id = "rubella"
display_name = "Rubella"
snomed_concept = "36653000"
snomed_description = "Rubella (disorder)"

[[antigen]]
id = "hpv"
display_name = "Human papillomavirus (HPV)"
snomed_concept = "240532009"
snomed_description = "Human papillomavirus infection (disorder)"
```

---

## Product Mapping File: `rules/product-map-uk-snomed-dm.toml`

FHIR records contain product codes (SNOMED CT). The mapping file bridges each product code to two things: its `product_class` (the conformance unit the Green Book names, used to match doses to series) and the `antigens` it covers (used for the disease-coverage view). See [conformance vs coverage](./conformance-vs-coverage.md).

This mapping is maintained using SNOMED CT ECL queries against the UK drug extension, which encodes the antigen composition of each product via the SNOMED concept hierarchy. For v1 a hand-curated table covering the ~10-15 products in the current schedule is sufficient.

The mapping is a separate file per coding system so it can be maintained independently and shared across jurisdictions using the same coding system.

When products change over time, a historical dose of the old product counts toward the antigens it actually covered, not those of the replacement. Antigen-level evaluation handles this naturally: a Pediacel (5-in-1) dose counts toward diphtheria/tetanus/pertussis/polio/Hib but not hepatitis B, even when the schedule that applies to the patient now expects 6-in-1. The test suite must exercise both the 5-in-1 → 6-in-1 transition and the MMRV → MMR case, where the non-overlapping varicella antigen makes the substitution lossy in the opposite direction.

```toml
# Product-to-antigen mapping for UK SNOMED drug extension codes.
# Maintained by querying the SNOMED CT UK drug extension hierarchy.
# Each entry maps a product SNOMED code to the antigens it covers.

coding_system = "snomed-uk-dm"
coding_system_url = "https://snomed.info/sct"
last_verified = "2026-01-01"

[[product]]
code = "39114911000001105"
display = "Infanrix Hexa vaccine (product)"
product_class = "6-in-1"
antigens = ["diphtheria", "tetanus", "pertussis", "polio", "hib", "hepatitis-b"]

[[product]]
code = "9743801000001106"
display = "Pediacel vaccine (product)"
product_class = "5-in-1"
antigens = ["diphtheria", "tetanus", "pertussis", "polio", "hib"]
notes = "5-in-1; does not include hepatitis B. Used before 2017 6-in-1 introduction."

[[product]]
code = "12672211000001104"
display = "Bexsero vaccine (product)"
antigens = ["meningococcal-b"]

[[product]]
code = "7374211000001102"
display = "Rotarix vaccine (product)"
antigens = ["rotavirus"]

[[product]]
code = "14473901000001100"
display = "Prevenar 13 vaccine (product)"
antigens = ["pneumococcal"]

[[product]]
code = "9684201000001108"
display = "Menitorix vaccine (product)"
antigens = ["hib", "meningococcal-c"]

[[product]]
code = "9324201000001104"
display = "Priorix vaccine (product)"
antigens = ["measles", "mumps", "rubella"]

[[product]]
code = "14491811000001103"
display = "Gardasil 9 vaccine (product)"
antigens = ["hpv"]

[[product]]
code = "22704311000001109"
display = "Revaxis vaccine (product)"
antigens = ["tetanus", "diphtheria", "polio"]
notes = "Td/IPV - used for teenage booster."
```

---

## FHIR Input Format

### Minimum viable input bundle

```json
{
  "resourceType": "Bundle",
  "type": "collection",
  "entry": [
    {
      "resource": {
        "resourceType": "Patient",
        "id": "patient-1",
        "birthDate": "2023-08-15"
      }
    },
    {
      "resource": {
        "resourceType": "Immunization",
        "status": "completed",
        "vaccineCode": {
          "coding": [{
            "system": "http://snomed.info/sct",
            "code": "39114911000001105",
            "display": "Infanrix Hexa vaccine (product)"
          }]
        },
        "patient": { "reference": "Patient/patient-1" },
        "occurrenceDateTime": "2023-10-16",
        "protocolApplied": [{
          "doseNumberPositiveInt": 1
        }]
      }
    }
  ]
}
```

Sources of FHIR vaccination records compatible with this format:

- NHS Digital Immunisation FHIR API ([`NHSDigital/immunisation-fhir-api`](https://github.com/NHSDigital/immunisation-fhir-api))
- NIMS (National Immunisation Management System) exports
- GP system exports (EMIS, SystmOne) via [GP Connect](https://digital.nhs.uk/services/gp-connect)

Multiple `Immunization` resources in a single bundle constitute the complete patient record. Records from different sources (GP, school immunisation, pharmacy) can be composed into a single bundle for evaluation.

---

## Evaluation Logic

### Eligibility check

Before evaluating a series for a patient, check:

1. `eligibility.population` - if `"all"`, proceed. If `"female"` or `"male"`, check patient sex from the `Patient` resource.
2. `eligibility.male_born_on_or_after` - if present and patient is male, check DOB >= this date. If DOB is before this date, mark series as `NotApplicable`.

When the FHIR `Patient.gender` field is `other` or `unknown`, the evaluator treats the patient as eligible for any sex-restricted series and attaches an uncertainty flag to the result so downstream consumers can see that the determination depends on a missing data point.

### Two questions: conformance and coverage

Evaluation answers two distinct questions, with two different matching rules. See [conformance vs coverage](./conformance-vs-coverage.md).

- **Schedule conformance** — "did the patient receive the doses the Green Book asked for, at valid ages and intervals?" The Green Book names *products* per appointment, so conformance matches a dose to a series by **product class**, not antigen overlap. This is what determines series completion and overall status below.
- **Antigen coverage** — "what diseases is the patient protected against?" Computed separately by aggregating the antigens of every product received, independent of series. (Deferred; the data model supports it additively.)

Matching by product class is what prevents a 6-in-1 dose — which contains Hib, tetanus, diphtheria and polio — from being incorrectly counted against the Hib/MenC or Td/IPV booster series.

### Dose validity (conformance)

A dose belongs to a series if the product's `product_class` equals the series' `product_class`. A matched dose is **within the standard schedule** if all of:

1. The date of administration is on or after `earliest_age` (calculated from DOB).
2. If `latest_age` is set, the date is on or before that age.
3. The interval from the previous valid dose is >= `min_interval_from_previous`.

A dose that fails any of these checks was still given - the FHIR record is evidence the event happened - but it falls **outside the standard schedule**. This can run in either direction: too early (before `earliest_age`, or short of the minimum interval) or too late (after a `latest_age` cutoff). Such a dose is recorded as received, labelled "outside standard schedule" with the specific reason, and does not count toward series completion. For doses with a hard `latest_age` cutoff - rotavirus being the clearest case, where late administration carries a real intussusception risk - not counting it is clinically required; for marginal cases the label is a flag for human review rather than a clinical ruling. "Outside standard schedule" is preferred over "invalid": the dose is a real clinical event, just not one that satisfies the course.

A dose whose product class matches no series in the applicable schedule (e.g. a 5-in-1 dose against a 6-in-1 schedule) conforms to nothing in that version and is surfaced as unmatched rather than counted.

### One product class, several series

A product class can serve more than one series - the clearest case is `MMR`, which maps to both the first-dose (`mmr-primary`) and second-dose (`mmr-second`) series. Such series are evaluated **as one programme**: the class's recorded doses are allocated across the programme's dose slots (ordered by target age) one dose per slot, in **date order**. So the earliest MMR dose fills MMR dose 1, the next fills MMR dose 2, and `min_interval_from_previous` for dose 2 is measured from dose 1. Matching each series independently against every dose of the class - which would flag a correctly-vaccinated child's dose 2 as an "extra" under the first-dose series and their dose 1 as "too early" under the second - is wrong, and is not what the engine does.

### Dose sequencing

Which physical dose is dose 1, dose 2, ...? Three signals can indicate it, none reliable alone across UK source systems:

- the **date** of administration (relative order);
- `protocolApplied.doseNumberPositiveInt` in the FHIR record;
- the SNOMED **procedure** code (in the [`UKCore-VaccinationProcedure`](https://fhir.hl7.org.uk/StructureDefinition/Extension-UKCore-VaccinationProcedure) extension, distinct from the dm+d *product* code in `vaccineCode`), whose concept can name the dose - e.g. "Administration of *second dose* of ...". Note the *first* dose is often recorded with the generic administration code, so the procedure code reliably indicates dose 2+ but not always dose 1.

The recorded dose number and procedure code are **entered by people**, who can get them wrong. So **date order is authoritative** for allocation, and the other two are cross-checks: a disagreement raises a soft `flag` on the `RecordedDose` (it does not affect validity) for human review, rather than overriding the dates.

### Duplicate doses ("echoes")

The Immunisation API draws on several upstream systems, and the same physical vaccination is frequently recorded **twice with different dates** - e.g. one system digitally notifies the dose, and a GP surgery separately keys it in by hand using the date the paper notification arrived. Counting both would inflate the course. Where two records carry the **same procedure code** they are taken to be the same act: the earliest is kept and the rest are reported as `duplicate_doses` rather than counted. (Records without a procedure code carry no duplicate signal and are all kept.)

### Series completion

Per series, against the doses defined for it:

- `Complete` - all expected doses received and valid
- `Partial` - at least one valid dose but fewer than expected
- `None` - no valid doses
- `NotApplicable` - patient not eligible for this series (see eligibility)

Each series is additionally annotated with whether each outstanding dose is **due** (its `earliest_age` is on or before `evaluated_at`) or **not yet due**. This is what separates "a dose that should have been given by now is missing" from "the patient simply hasn't reached the age for the next dose", and it drives the headline status below.

### Overall status

"Is this person fully vaccinated?" is ambiguous, so evaluation reports two complementary determinations.

**Up-to-date for age** is the headline, age-relative answer - "are there gaps that should have been filled by now?":

- `UpToDateForAge` - every dose that is *due* by `evaluated_at` has been received and is valid; doses not yet due are not held against the patient
- `BehindForAge` - at least one *due* dose is missing, or is only satisfied by a dose given outside the standard schedule (this is the case the retired `PartiallyVaccinated` used to cover)
- `Unvaccinated` - no valid doses recorded at all
- `Unknown` - patient DOB missing, or no records and the evaluator cannot distinguish genuinely unvaccinated from absent data

**Fully vaccinated** is a strict, age-independent flag retained alongside the headline status: true only when every applicable series is `Complete` - every dose at every age received and valid. A correctly-vaccinated 6-month-old is `UpToDateForAge` but not yet fully vaccinated; an adult who completed the whole childhood schedule is both. Because "fully vaccinated" is loose clinical shorthand for "up to date", it is reported as an explicit, precisely-defined flag rather than as the headline term. We deliberately do *not* model a "fully immunised for life stage" status: with new vaccines added across the life course (shingles, pneumococcal for older adults, and so on) that target is a moving one requiring constant maintenance, whereas "all doses in this schedule version" is bounded and stable.

Between them these cover consumer need (1) "is this child correctly vaccinated for their age?" (the headline status) and need (2) "what are the specific gaps?" (the per-series breakdown with due/not-yet-due annotations). Two further consumer needs are foreseen but out of scope for v1 - (3) a predicted future vaccination schedule and (4) detection of record errors such as duplicates - and are tracked on the [roadmap](roadmap.md).

---

## Future extensions (designed for, not v1)

### At-risk and overriding rules

The current schedule has additional or extended series for immunocompromised children, premature infants, hepatitis B carrier contacts, and similar groups. These are out of scope for v1, but the `eligibility` structure should not preclude them. The intended model is a numerical priority on each rule, analogous to MX records in DNS: the primary schedule applies by default, and a higher-priority at-risk rule whose eligibility predicate matches overrides the primary rule for that patient. This keeps the primary schedule readable in isolation and avoids tangling at-risk logic into the default path.

### Catch-up schedules

When a patient presents late (for example a 3-year-old with no previous vaccinations), the catch-up schedule differs from the primary one. v1 evaluates only against the primary schedule and flags incomplete series. The `eligibility` structure must accommodate catch-up rules in a future version without a rewrite, and the v1 test suite includes catch-up scenarios so the structure is exercised against realistic inputs from the outset.

---

## Historical Versioning (v2 - deferred but designed for)

The file-per-version approach means historical evaluation is an additive feature:

1. Parse patient DOB from the FHIR bundle
2. Call `load_schedule_for_date(rules_dir, "uk", dob)` which selects the schedule file (e.g. `rules/schedule-uk-*.toml`) where `valid_from <= dob` and no successor has `valid_from <= dob`
3. Proceed with the same evaluation logic

Schedule files for historical versions would be curated manually, working back from the current schedule using Green Book chapter revisions and JCVI/DoH publications as sources. The change history on the GOV.UK Green Book Chapter 11 page provides a useful skeleton for reconstruction.

Likely scope for full historical coverage: approximately 1990 to present, covering roughly 8-12 distinct schedule versions. This is a bounded, tractable curation task.
