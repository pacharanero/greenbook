# Roadmap

Where greenbook is and where it goes next. This tracks the gap between the [specification](./spec/) and the implementation, and sequences the remaining work. It is a living document — edit it as decisions land. Open design questions live in [queries.md](./queries.md); answers there feed back into the spec, and the resulting work lands here.

## Status at a glance

The v1 POC core is working and green: `cargo test` passes, `cargo clippy --all-targets -- -D warnings` is clean. The engine parses a FHIR R4 bundle, loads a TOML schedule and product map, and produces a per-series and overall classification with a human-readable and JSON report. One schedule version (`gb/2026-01-01.toml`), one product map, and one fixture are bundled.

What this means: the file format and evaluation pipeline are proven end-to-end on the happy path. The remaining v1 work is correctness (eligibility, dose-matching), completeness (the other CLI commands and the `by_antigen` breakdown), and breadth of test coverage — none of it requires rearchitecting.

Nothing is committed to git yet. The first milestone is to get a clean, reviewed baseline committed.

## Guiding principle

Build the format and engine correctly for the *current* schedule so that historical versioning (v2), at-risk overrides, and catch-up rules become additive extensions rather than rewrites. Every design choice made now is judged against that test. See [spec/introduction.md](./spec/introduction.md) §"Scope for v1".

## Milestones

### M0 — Baseline commit and CI

Get the working POC onto a reviewable footing before adding behaviour.

- [ ] Initial git commit of the current tree
- [ ] `CHANGELOG.md` (referenced by the schedule `change_summary` but absent)
- [ ] CI workflow running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` (per [spec/rust-impl.md](./spec/rust-impl.md) §"Build and Tooling Conventions"; confirm latest stable action versions before pinning)

### M1 — Resolve the open design questions

These block clean test fixtures and correct output. They are decisions, not code — see [queries.md](./queries.md). Each needs a ruling before the dependent code is worth writing.

- [ ] §1 Dose-to-series matching strictness (proposed: stricter superset matching, with greedy assignment as tiebreaker) — **this gates M2 and the fixture set**
- [ ] §2 "Fully vaccinated" vs "up-to-date for age" — pick a status model
- [ ] §3 Suppressing irrelevant invalid doses in the report (largely falls out of §1)
- [ ] §5 `latest_age` semantics for late-but-given doses

### M2 — Correctness gaps in the engine

The engine currently takes the happy path. Close the known divergences from [spec/standard.md](./spec/standard.md) §"Evaluation Logic".

- [ ] **Enforce eligibility.** `population` and `male_born_on_or_after` are parsed but never checked, so no series is ever `NotApplicable` and HPV's sex restriction is inert (queries §4). Wire in the eligibility check, including the `gender = other|unknown` → eligible-with-uncertainty-flag rule.
- [ ] Apply the §1 matching ruling so 6-in-1 doses stop being falsely flagged INVALID under booster series.
- [ ] Add the `UpToDateForAge` status (or chosen §2 model) and the per-series due / not-yet-due annotation.
- [ ] **Dose-sequence cross-check.** Derive sequence from dates (current behaviour) but cross-check against `protocolApplied.doseNumberPositiveInt` and SNOMED signals, flagging discrepancies on the `RecordedDose` rather than silently preferring dates ([spec/standard.md](./spec/standard.md) §"Dose sequencing").
- [ ] **Unknown product-code warning.** A vaccine code absent from the product map currently disappears silently from evaluation (see [docs/testing.md](./docs/testing.md) §7). Surface it.

### M3 — Output completeness

Bring the output up to the specced shape.

- [ ] `by_antigen` breakdown (`AntigenStatus`) on `VaccinationStatus` — specced in [spec/rust-impl.md](./spec/rust-impl.md), not yet implemented.
- [ ] Reconcile types with the spec where they have drifted (e.g. `AgeOffset` vs a separate `Interval`; whether `AgeOffset` needs `Ord` — `render` will require sorting by age).

### M4 — The rest of the CLI

Only `evaluate` exists. The other commands are specced in [spec/rust-impl.md](./spec/rust-impl.md) §"CLI".

- [ ] `validate <schedule>` — structural + referential + logical-consistency checks (sequential dose numbers, interval present where dose > 1). Natural next command; reuses existing validation.
- [ ] `render <schedule> [--format table|markdown|html]` — series-centric → age-centric pivot. **This is the command that demonstrates the core thesis: the publication PDF generated from the data, not the reverse.**
- [ ] `versions [--country <code>]` — list available schedule versions.
- [ ] `diff <a> <b> [--format table|json]` — compare two schedule versions for PR review.

### M5 — Test coverage

The spec lists nine fixtures ([spec/rust-impl.md](./spec/rust-impl.md) §"Crate Structure"); one exists. Build them out — several deliberately exercise structures that v2 features will lean on.

- [ ] `fully_vaccinated`, `missing_menb`, `unvaccinated`, `partial_hpv`
- [ ] `sex_unknown_hpv` (depends on M2 eligibility)
- [ ] `dose_sequence_mismatch` (depends on M2 cross-check)
- [ ] `product_5in1_to_6in1` — historical Pediacel dose vs current 6-in-1 schedule
- [ ] `product_mmrv_to_mmr` — lossy substitution in the opposite direction; **requires adding the MMRV product and a `varicella` antigen** to the registry/map ([spec/standard.md](./spec/standard.md) §"Product Mapping File")
- [ ] `catch_up_age_3` — late presenter; exercises the eligibility structure against the catch-up case ahead of M-future
- [ ] Fold the [test-data/](./test-data/) MMR catch-up scenarios into integration tests

## Deferred (designed for, explicitly not v1)

These are out of scope now but the format and engine must not preclude them — that is the whole point of the v1 design discipline.

- **Historical versioning (v2).** `load_schedule_for_date(dir, country, date)` selecting the schedule where `valid_from <= dob` with no nearer successor. Then curate ~8–12 historical GB versions back to ~1990. See [spec/standard.md](./spec/standard.md) §"Historical Versioning" and [spec/introduction.md](./spec/introduction.md).
- **At-risk / overriding rules.** DNS-MX-style numerical priority on eligibility rules; higher-priority matching rule overrides the primary schedule. [spec/standard.md](./spec/standard.md) §"Future extensions".
- **Catch-up schedules.** Distinct from primary-schedule evaluation; v1 only flags incomplete series.
- **Multi-jurisdiction.** The `schedules/<country>/` and `products/<coding-system>.toml` layout is already shaped for `us`/`au`; no non-GB data yet.

## Suggested near-term order

M0 (commit + CI) → settle M1 §1 (matching) since it gates everything downstream → M2 eligibility (highest-impact correctness gap) → M4 `validate` then `render` (cheap, high-signal, and `render` proves the thesis) → backfill M5 fixtures alongside.
