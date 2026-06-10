# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This changelog tracks the **crate and tooling**. Changes to schedule *content* are recorded per-version in each schedule file's `change_summary` and in the git history of `schedules/`.

## [Unreleased]

### Changed

- Conformance matching is now by **product class** rather than antigen overlap, separating "schedule conformance" from "antigen coverage" ([ADR 0001](./docs/adr/0001-product-class-conformance-vs-antigen-coverage.md)). Product map entries and schedule series now carry a required `product_class` field. This fixes 6-in-1 doses being falsely flagged INVALID under the Hib/MenC and Td/IPV booster series.
- Schedule and product-map files moved to a flat, jurisdiction-prefixed layout (`schedules/uk-2026-01-01.toml`, `products/uk-snomed-dm.toml`) and the jurisdiction code set to `UK` (ISO 3166 exceptionally-reserved, chosen over `GB` so the UK-wide scope incl. Northern Ireland is unambiguous).
- Spec now defines the resolved status model — `UpToDateForAge` as the headline status alongside a strict `fully_vaccinated` flag — and the "outside standard schedule" terminology for doses given too early or too late. These are folded into the spec; the engine implementation is tracked on the [roadmap](./spec/roadmap.md) (M2).

- A product class that maps to several series (e.g. `MMR` → first- and second-dose series) is now evaluated as one **programme**: doses are allocated across the series' slots in date order rather than matched against each series independently. This removes spurious "extra dose" / "too early" flags for a child with both MMR doses. **Dose sequence** is taken from date order, with the recorded `protocolApplied` dose number and the SNOMED procedure code (UKCore-VaccinationProcedure extension) as cross-checks that raise a soft flag on disagreement.

### Added

- Initial proof-of-concept: FHIR R4 bundle parser, TOML schedule and product-map loaders, age-offset arithmetic, and a per-series + overall vaccination-status evaluation engine.
- Duplicate ("echo") detection: records sharing a SNOMED procedure code are treated as the same act; the earliest is kept and the rest reported in `duplicate_doses`. The FHIR parser reads the `UKCore-VaccinationProcedure` extension (procedure code + display).
- Demo: a "Build your own" custom-patient mode, and presets exercising the MMR allocation, duplicate echoes, and a mis-keyed dose number. A list of FHIR test-data sources in the README.
- CLI `evaluate` subcommand with human-readable report and JSON output.
- Current UK schedule (`schedules/uk-2026-01-01.toml`) and UK SNOMED drug-extension product map (`products/uk-snomed-dm.toml`).
- Specification documents under `spec/`, POC walkthrough in `docs/testing.md`, and a [roadmap](./spec/roadmap.md).
- CI running `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
