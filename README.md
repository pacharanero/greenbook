# greenbook

greenbook evaluates a patient's FHIR vaccination history against a computable, versioned representation of the NHS routine childhood immunisation schedule (the [Green Book](https://www.gov.uk/government/publications/immunisation-schedule-the-green-book-chapter-11)), reporting whether they are up-to-date for their age, which doses are missing, and (strictly) whether every dose the schedule defines has been given - per series and in aggregate.

For the UK, the childhood vaccination schedule is currently published as human-readable PDF files that serve as the **primary source of truth**. As of 2026 there is no computable version of the schedule. This prototype is an attempt to explore a proof-of-concept for a **computable schedule format** and **evaluation engine**, with the long-term goal of creating a versioned, computable Green Book that can be maintained in parallel with the human-readable version and eventually replace it as the upstream primary publication and 'source of truth', with downstream PDFs being generated from the computable version rather than the other way around.

### What is the long-term potential of getting this right?

The long-term goal is that vaccination and public health experts could eventually author schedule changes **directly** in a computable format, and all downstream publications - PDFs, websites, and of course clinical digital tools - would be generated from this one 'upstream' and trusted computable source. It would replace the current 'digital paper' workflow where a PDF is published from a Word document and clinical code has to be written as a (potentially inaccurate) reverse-engineered derivative of it.

## Repository

The canonical, language-neutral material lives at the top level; each implementation is a peer folder.

| Path | What |
| --- | --- |
| [`spec/`](./spec/) | The specification - formats, evaluation semantics, [ubiquitous language](./spec/ubiquitous-language.md), [roadmap](./spec/roadmap.md). Language-neutral. |
| [`schedules/`](./schedules/), [`products/`](./products/) | The canonical computable Green Book sources (TOML). |
| [`conformance/`](./conformance/) | The shared test harness: fixtures, a case manifest, and golden outputs every implementation is validated against. |
| [`rust/`](./rust/) | The **reference** implementation (engine + CLI), and the generator of the conformance goldens. |
| [`js/`](./js/) | The JavaScript implementation (also powers the demo). |
| [`docs/`](./docs/) | The [presentation](./docs/presentation/) and the interactive [demo](./docs/demo/). |

Both implementations are independent ports of the spec, kept in step by the conformance suite - so each can be developed and validated on its own while staying behaviourally identical. More implementations (Ruby, Python) can join by running the same suite. See each folder's README to get started.

---

## Walkthrough

This walks through everything from installation to a working demonstration of the evaluator's features. Every command is run from the repository root.

This drives the **reference (Rust) implementation**'s CLI. For the JavaScript implementation see [js/README.md](./js/README.md); both are validated by the shared [conformance suite](./conformance/).

### Prerequisites

- A [Rust](https://rustup.rs/) toolchain (edition 2021; `cargo`/`rustc` 1.93+ tested).
- `git`.

### 1. Get the code and build

```sh
git clone https://github.com/pacharanero/greenbook.git
cd greenbook
cargo build --manifest-path rust/Cargo.toml
```

The first build pulls a handful of crates (chrono, serde, toml, serde_json, clap, thiserror) and takes a few seconds; later builds are incremental.

### 2. Run the test suite

```sh
cargo test --manifest-path rust/Cargo.toml
```

You should see the unit tests (age parsing and date arithmetic), the integration tests, and the conformance test (which checks the engine reproduces the golden outputs in `conformance/expected/`) all pass.

### 3. The `evaluate` command

The evaluator takes three inputs - a schedule, a product map, and a patient's FHIR bundle - and prints a report:

```sh
cargo run --bin greenbook -- evaluate <schedule.toml> <product-map.toml> <bundle.json> \
  [--evaluated-at YYYY-MM-DD] [--format report|json]
```

To get a `greenbook` binary on your `PATH` instead of using `cargo run`, install it: `cargo install --path .`, then call `greenbook evaluate ...` directly.

The bundled inputs are:

- `schedules/uk-2026-01-01.toml` - the current UK schedule
- `products/uk-snomed-dm.toml` - the SNOMED UK drug-extension product → class/antigen map
- `conformance/fixtures/*.json` - the demonstration patients used below

The examples below pass `--evaluated-at 2026-04-29` so the output is deterministic regardless of today's date. Omit it to evaluate as of today.

### Demo 1 - up to date for age

A 6-month-old who has had every dose due so far. The headline answer is **up to date for age**, even though later doses (MMR, HPV, the boosters) have not been given - they are not due yet.

```sh
cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- evaluate \
  schedules/uk-2026-01-01.toml products/uk-snomed-dm.toml \
  conformance/fixtures/six-month-fully-vaccinated.json --evaluated-at 2026-04-29
```

```
Up-to-date status: UP_TO_DATE_FOR_AGE
Fully vaccinated:  no (strict: every eligible series complete)

By series:
---------
  [COMPLETE   ] 6-in-1 (3/3 due, 3 total) - up to date
  [COMPLETE   ] Rotavirus (2/2 due, 2 total) - up to date
  [PARTIAL    ] MenB (2/2 due, 3 total) - up to date
  [PARTIAL    ] PCV (pneumococcal) (1/1 due, 2 total) - up to date
  [NONE       ] Hib/MenC booster (0/0 due, 1 total) - up to date
  [NONE       ] MMR (first dose) (0/0 due, 1 total) - up to date
  ... (further not-yet-due series) ...
```

Note the two distinct answers: the patient is **up to date for age**, but not **fully vaccinated** in the strict "every dose at every age" sense - that is correct for a 6-month-old. `n/m due, k total` reads as "n valid doses out of m due so far, k in the whole series".

### Demo 2 - behind for age, with the specific gaps

An 18-month-old who had the primary infant doses but missed every 12-month appointment. The headline flips to **behind for age**, and the per-series breakdown shows exactly which doses are overdue.

```sh
cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- evaluate \
  schedules/uk-2026-01-01.toml products/uk-snomed-dm.toml \
  conformance/fixtures/behind-for-age-toddler.json --evaluated-at 2026-04-29
```

```
Up-to-date status: BEHIND_FOR_AGE

By series:
---------
  [COMPLETE   ] 6-in-1 (3/3 due, 3 total) - up to date
  [COMPLETE   ] Rotavirus (2/2 due, 2 total) - up to date
  [PARTIAL    ] MenB (2/3 due, 3 total) - BEHIND
  [PARTIAL    ] PCV (pneumococcal) (1/2 due, 2 total) - BEHIND
  [NONE       ] Hib/MenC booster (0/1 due, 1 total) - BEHIND
  [NONE       ] MMR (first dose) (0/1 due, 1 total) - BEHIND
  ... (teenage series, not yet due) ...
```

The series marked `BEHIND` are the catch-up worklist: MenB dose 3, PCV dose 2, the Hib/MenC booster, and MMR dose 1.

### Demo 3 - doses given outside the standard schedule

Doses that were given but break an age or interval rule are recorded and labelled **outside standard schedule** rather than silently passed or harshly called "invalid". Here a 6-in-1 second dose is given a week too soon, and a rotavirus dose is given after its hard cutoff.

```sh
cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- evaluate \
  schedules/uk-2026-01-01.toml products/uk-snomed-dm.toml \
  conformance/fixtures/out-of-schedule-doses.json --evaluated-at 2026-04-29
```

```
  [PARTIAL    ] 6-in-1 (1/3 due, 3 total) - BEHIND
      - ok              2025-12-24  dose 1  (1mo 3w 4d)  [Infanrix Hexa vaccine (product)]
      - OUT-OF-SCHEDULE  2025-12-31  dose 2  (2mo 2d)  [Infanrix Hexa vaccine (product)]
          ! given before earliest_age 10 weeks (2026-01-07) - outside standard schedule
          ! interval from previous dose < 4 weeks (needs to be on/after 2026-01-21) - outside standard schedule
  [NONE       ] Rotavirus (0/2 due, 2 total) - BEHIND
      - OUT-OF-SCHEDULE  2026-03-01  dose 1  (4mo 1d)  [Rotarix vaccine (product)]
          ! given after latest_age 14 weeks 6 days (2026-02-10) - outside standard schedule
```

### Demo 4 - doses that match no series

A record can contain doses that belong to no series in the loaded schedule - an unknown product code, or a known product whose class the current schedule no longer uses (a Pediacel 5-in-1 dose against the 2026 6-in-1 schedule). These are surfaced in their own section instead of vanishing.

```sh
cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- evaluate \
  schedules/uk-2026-01-01.toml products/uk-snomed-dm.toml \
  conformance/fixtures/unmatched-doses.json --evaluated-at 2026-04-29
```

```
Unmatched doses:
---------------
  - 2026-01-21  [Pediacel vaccine (product)]  (product class "5-in-1" has no series in this schedule version)
  - 2026-01-21  [Unknown investigational vaccine]  (unknown product code (not in the product map))
```

### Machine-readable output

Pass `--format json` to any of the above for the full structured result (every series, every recorded dose with its `within_schedule` flag and notes, and the unmatched doses) suitable for piping into other tools:

```sh
cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- evaluate \
  schedules/uk-2026-01-01.toml products/uk-snomed-dm.toml \
  conformance/fixtures/six-month-fully-vaccinated.json --evaluated-at 2026-04-29 --format json
```

---

## Problem Statement

### Why this is non-trivial

To determine whether a person is "fully vaccinated" you need to know:

1. What schedule applied when they were born (and what was in scope by the time each dose was due)
2. What they actually received
3. Whether what they received satisfies the schedule that applied to them

The NHS childhood schedule has changed substantially and repeatedly. A child born in 1998 has a genuinely different "complete" schedule than one born in 2010 or 2020. Any system that applies today's schedule to all patients will over- or under-flag.

Examples of schedule changes:

- Hib introduced 1992
- MenC introduced 1999
- PCV (pneumococcal) introduced 2006; schedule changed again in 2020 (3+0 to 2+1)
- Rotavirus introduced 2013
- MenB introduced 2015
- HPV introduced 2008 for girls only; extended to boys 2019
- 5-in-1 replaced by 6-in-1 in 2017 (adding hepatitis B)
- Flu extended incrementally to primary school age from ~2013

### Historical versioning is the hard part

This is a **valid-time** data problem. The schedule is not a single document but a series of versioned snapshots, each with a `valid_from` date. Evaluating a patient born in 2003 requires the schedule as it stood in 2003, not today's schedule.

No open-source project anywhere in the world has solved this for a national schedule in a principled, versioned way. The closest existing work is the US CDC's **[CDSi](https://www.cdc.gov/vaccines/programs/iis/interop-proj/cds.html)** (Clinical Decision Support for Immunization) specification and the **[ICE](https://github.com/cdsframework/ice)** (Immunization Calculation Engine) open-source implementation - but both are US-only (ACIP schedule) and both are current-schedule-forward systems, not historically aware.

**The computable, versioned Green Book does not exist. This project creates it.**

### Scope for v1

Start with the current schedule only. Build the evaluation engine and file format correctly from the outset so that historical versioning is an additive extension, not a rewrite. The data management and format design choices made now determine whether the historical problem is tractable later.

---

## Prior Art

### ICE - Immunization Calculation Engine

- GitHub: [`cdsframework/ice`](https://github.com/cdsframework/ice)
- Maintained by NYC Dept of Health and HLN Consulting
- Free and open source ([LGPL-3.0](https://www.gnu.org/licenses/lgpl-3.0.html))
- Docker image available; actively maintained as of 2026
- ACIP (US) schedule only; current-schedule-forward
- Architecture is worth studying: separates schedule data (per-antigen configuration files) from the evaluation engine ([Drools](https://www.drools.org/) rules)
- Recognised as a Digital Public Goods Standard by the [DPGA](https://digitalpublicgoods.net/) (2021)

### CDC CDSi

- Published by [CDC](https://www.cdc.gov/vaccines/programs/iis/interop-proj/cds.html) as an implementation-neutral specification
- Logic Specification (PDF) plus Supporting Data (XML/Excel per antigen)
- Versioned document (currently v4.6) updated after each [ACIP](https://www.cdc.gov/vaccines/acip/index.html) recommendation
- Useful as a design pattern reference; not directly applicable to UK

### UK NHS - what exists

- [Green Book Chapter 11](https://www.gov.uk/government/publications/immunisation-schedule-the-green-book-chapter-11): the authoritative UK schedule, published as versioned PDFs on [GOV.UK](https://www.gov.uk/). Change history visible in page revision notes but not structured data.
- [`NHSDigital/immunisation-fhir-api`](https://github.com/NHSDigital/immunisation-fhir-api): NHS Digital's FHIR R4 Immunisation API (Python, AWS Lambda backend). Handles CRUD for vaccination records. No schedule evaluation logic.
- [`NHSDigital/FHIR-R4-UKCORE-STAGING-MAIN`](https://github.com/NHSDigital/FHIR-R4-UKCORE-STAGING-MAIN): UK Core FHIR profiles including `UKCore-Immunization`.
- No open-source computable UK schedule exists.

---

## Test data sources

Realistic FHIR immunisation test data is genuinely scarce. These are the sources we have found useful; they also document how the real data carries the **dose sequence** (which matters for matching - see the engine's dose-sequencing logic).

- **NHS sandbox example payloads** - [`NHSDigital/immunisation-history-api`](https://github.com/NHSDigital/immunisation-history-api), under `sandbox/immunization-handler/v1/fhir-responses/` (`covid`, `flu`, `hpv`, `empty`). Ready-made `Immunization` resources, complete with the `UKCore-VaccinationProcedure` extension and `protocolApplied`. The most directly reusable - our fixtures follow their shape.
- **NHS Immunisation History test data pack** - [test data page](https://digital.nhs.uk/developer/api-catalogue/immunisation-history-fhir/immunisation-history-fhir-api-test-data): ~120 synthetic patients with varied histories (two-doses-plus-booster, two-doses-no-booster, first-dose-only, none), each event carrying date / **procedure** / product. Aligned to the [PDS FHIR API](https://digital.nhs.uk/developer/api-catalogue/personal-demographics-service-fhir) test patients (same NHS numbers). Lives in the integration environment as a spreadsheet, not downloadable JSON.
- **UK Core / NHS API examples** - the [Immunisation FHIR API](https://digital.nhs.uk/developer/api-catalogue/immunisation-fhir-api) and the (deprecated) [NHS England FHIR examples on Simplifier](https://simplifier.net/guide/NHSDigital/Home/Examples/AllExamples/Immunization), plus [GP Connect Immunization examples](https://developer.nhs.uk/apis/gpconnect-1-3-0/accessrecord_structured_development_fhir_examples_immunizations.html).
- **SNOMED codes** - dose sequence is carried as a SNOMED **procedure** code in the [`UKCore-VaccinationProcedure`](https://fhir.hl7.org.uk/StructureDefinition/Extension-UKCore-VaccinationProcedure) extension (distinct from the dm+d **product** code in `vaccineCode`). UK MMR/MMRV procedure + product codes: [NHS Networks](https://networks.nhs.uk/blog/snomed-procedure-and-product-codes-for-mmrv-vaccines/); the dose-coding pattern is described in the [SNOMED COVID-19 implementation guide](https://docs.snomed.org/implementation-guides/snomed-ct-guide-for-covid-19/2-coding-covid-19-related-data/2.5-prevention-treatment-and-education). Note: the *second*/*third* dose usually has an explicit procedure code, while the *first* dose is often just the generic administration code.

Two real-world caveats these sources reflect, both handled by the engine: the dose number in `protocolApplied`/the procedure code is **entered by humans and can be wrong**, so date order is authoritative and these are cross-checks; and the same physical vaccination is often recorded **twice from different systems with different dates** ("echoes"), detectable because both carry the same procedure code.
