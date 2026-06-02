# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This changelog tracks the **crate and tooling**. Changes to schedule *content* are recorded per-version in each schedule file's `change_summary` and in the git history of `schedules/`.

## [Unreleased]

### Added

- Initial proof-of-concept: FHIR R4 bundle parser, TOML schedule and product-map loaders, age-offset arithmetic, and a per-series + overall vaccination-status evaluation engine.
- CLI `evaluate` subcommand with human-readable report and JSON output.
- Current GB schedule (`schedules/gb/2026-01-01.toml`) and UK SNOMED drug-extension product map (`products/gb-snomed-dm.toml`).
- Specification documents under `spec/`, POC walkthrough in `docs/testing.md`, and a [roadmap](./roadmap.md).
- CI running `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
