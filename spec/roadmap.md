# Roadmap

Where greenbook is and where it goes next. This tracks the gap between the [specification](./) and the implementation, and sequences the remaining work. It is a living document — edit it as decisions land. The design questions raised during the POC are resolved and recorded in M1 below; their answers feed back into the spec, and the resulting work lands in the later milestones.

## Status at a glance

The v1 POC core is working and green: `cargo test` passes, `cargo clippy --all-targets -- -D warnings` is clean. The engine parses a FHIR R4 bundle, loads a TOML schedule and product map, and produces the up-to-date-for-age status, per-series breakdown, out-of-schedule flags, and unmatched-dose list, as a human-readable or JSON report. One schedule version (`uk-2026-01-01.toml`), one product map, and four demonstration fixtures are bundled (see the README [Walkthrough](../README.md#walkthrough)).

What this means: the file format and evaluation pipeline are proven end-to-end, and the M1 decisions are now implemented in the engine. The remaining v1 work is the dose-sequence cross-check, the other CLI commands and the `by_antigen` breakdown, and broader test coverage — none of it requires rearchitecting.

The baseline is committed and CI is green (M0 done), the POC design questions are resolved (M1), and most of M2 is implemented. The remaining work is M2's dose-sequence cross-check, then M3/M4.

## Guiding principle

Build the format and engine correctly for the *current* schedule so that historical versioning (v2), at-risk overrides, and catch-up rules become additive extensions rather than rewrites. Every design choice made now is judged against that test. See the [README](../README.md) §"Scope for v1".

## Milestones

### M0 — Baseline commit and CI

Get the working POC onto a reviewable footing before adding behaviour.

- [x] Initial git commit of the current tree
- [x] `CHANGELOG.md` (referenced by the schedule `change_summary` but absent)
- [x] CI workflow running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` (per [spec/rust-impl.md](./rust-impl.md) §"Build and Tooling Conventions"; confirm latest stable action versions before pinning)

### M1 — Resolve the open design questions

These were decisions, not code — each needed a ruling before the dependent code was worth writing. All are now resolved; the rulings are recorded below and folded into the spec.

- [x] §1 Dose-to-series matching — **resolved**: product-class conformance matching vs antigen coverage, [ADR 0001](../docs/adr/0001-product-class-conformance-vs-antigen-coverage.md). Implemented.
- [x] §3 Suppressing irrelevant invalid doses — **resolved**: falls out of §1; no suppression needed.
- [x] §2 "Fully vaccinated" vs "up-to-date for age" — **resolved**: `UpToDateForAge` (`BehindForAge`/`Unvaccinated`/`Unknown`) is the headline status; a strict `fully_vaccinated` flag is retained alongside. "Fully immunised for life stage" rejected as a moving target. See [spec/standard.md](./standard.md) §"Overall status".
- [x] §5 `latest_age` semantics for late-but-given doses — **resolved**: a dose breaking an age/interval rule (too early *or* too late) is recorded as received but labelled "outside standard schedule" and does not count toward completion. See [spec/standard.md](./standard.md) §"Dose validity".
- [x] §4 HPV sex restriction — settled in the spec (gender `other`/`unknown` → eligible + uncertainty flag); implementation pending in M2.
- [x] §6 crate/binary name — `greenbook` for both (verified free on crates.io 2026-06-05).
- [x] §7 schedule directory layout — flat `schedules/uk-<YYYY-MM-DD>.toml` until a second jurisdiction exists; jurisdiction code `UK`. See [spec/standard.md](./standard.md) §"Directory structure".

### M2 — Correctness gaps in the engine

The engine currently takes the happy path. Close the known divergences from [spec/standard.md](./standard.md) §"Evaluation Logic".

- [x] Product-class conformance matching ([ADR 0001](../docs/adr/0001-product-class-conformance-vs-antigen-coverage.md)) — 6-in-1 doses no longer falsely flagged under booster series.
- [x] **Enforce eligibility.** `population` and `male_born_on_or_after` are now checked: ineligible series are `NotApplicable` and excluded from the overall status, and a `gender = other|unknown` patient on a sex-restricted series is treated as eligible with an `eligibility_uncertain` flag (M1 §4).
- [x] **Adopt the resolved status model (§2).** `OverallStatus` is now the headline age-relative enum `UpToDateForAge`/`BehindForAge`/`Unvaccinated`/`Unknown`, `VaccinationStatus` carries the strict `fully_vaccinated: bool`, and each series carries `doses_due` and `up_to_date_for_age`. See [spec/standard.md](./standard.md) §"Overall status".
- [x] **Out-of-schedule labelling (§5).** `RecordedDose` now uses `within_schedule`/`schedule_notes`; rule-breaking doses (too early or too late) are reported as "outside standard schedule" rather than "invalid".
- [x] **Unmatched-dose reporting.** `VaccinationStatus.unmatched_doses` surfaces both an *unknown* product code and a *known* product whose class matches no series in the loaded schedule (e.g. 5-in-1 vs the 2026 schedule).
- [ ] **Dose-sequence cross-check.** Derive sequence from dates (current behaviour) but cross-check against `protocolApplied.doseNumberPositiveInt` and SNOMED signals, flagging discrepancies on the `RecordedDose` rather than silently preferring dates ([spec/standard.md](./standard.md) §"Dose sequencing").

### M3 — Output completeness

Bring the output up to the specced shape.

- [ ] **Antigen-coverage view** (`by_antigen` / `AntigenStatus`) — the "what diseases is this child protected against?" computation, deliberately deferred when [ADR 0001](../docs/adr/0001-product-class-conformance-vs-antigen-coverage.md) split conformance from coverage. Aggregates the `antigens` of every product received, independent of series.
- [ ] Reconcile types with the spec where they have drifted (e.g. `AgeOffset` vs a separate `Interval`; whether `AgeOffset` needs `Ord` — `render` will require sorting by age).
- [ ] **Predicted future schedule** (consumer need 3 from M1 §2). Project the doses a patient has not yet reached the age for, as a forward-looking "what's next and when" list (assuming no schedule change). Useful to parents and planners.
- [ ] **Record-error detection** (consumer need 4 from M1 §2). Surface likely data errors in the record itself — duplicate doses, implausibly-spaced administrations, doses before birth — distinct from schedule non-conformance.

### M4 — The rest of the CLI

Only `evaluate` exists. The other commands are specced in [spec/rust-impl.md](./rust-impl.md) §"CLI".

- [ ] `validate <schedule>` — structural + referential + logical-consistency checks (sequential dose numbers, interval present where dose > 1). Natural next command; reuses existing validation.
- [ ] `render <schedule> [--format table|markdown|html]` — series-centric → age-centric pivot. **This is the command that demonstrates the core thesis: the publication PDF generated from the data, not the reverse.**
- [ ] `versions [--country <code>]` — list available schedule versions.
- [ ] `diff <a> <b> [--format table|json]` — compare two schedule versions for PR review.

### M5 — Test coverage

Four demonstration fixtures are now bundled and covered by integration tests: `six-month-fully-vaccinated` (up-to-date for age), `behind-for-age-toddler` (behind, with gaps), `out-of-schedule-doses` (§5 labelling), and `unmatched-doses` (unknown + superseded products). Remaining fixtures from the spec's list — several deliberately exercise structures that v2 features will lean on:

- [ ] `partial_hpv` and an explicit `unvaccinated` older child
- [ ] `sex_unknown_hpv` — exercises the M2 `eligibility_uncertain` flag
- [ ] `dose_sequence_mismatch` (depends on the M2 dose-sequence cross-check)
- [ ] `product_5in1_to_6in1` — historical Pediacel dose vs current 6-in-1 schedule
- [ ] `product_mmrv_to_mmr` — lossy substitution in the opposite direction; **requires adding the MMRV product and a `varicella` antigen** to the registry/map ([spec/standard.md](./standard.md) §"Product Mapping File")
- [ ] `catch_up_age_3` — late presenter; exercises the eligibility structure against the catch-up case ahead of M-future
- [ ] Fold the [test-data/](../test-data/) MMR catch-up scenarios into integration tests

### Demonstration and documentation

* [ ] Reveal.js presentation of the entire 'thought chain' of the project - explaining to a general clinical/ttechnical audience the chain from the current Green Book state of affairs through the concepts of schedule, products, antigens, and coverage/conformance, using our ubiquitous language throughout. Use the /revealjs skill to make this.
* [ ] Web-based demo which allows the user to set the patient's DOB, select doses from a menu of products, and see the evaluation report update in real time. This is the most direct way to demonstrate the value of the computable schedule and engine to a non-technical audience. The demo should have some 'presets' which help to demonstrate the various features and edge cases of the engine.

### M6 — Schedule content gaps

Gaps in the *data* (not the engine) found under programmatic scrutiny. The Green Book has never been machine-checked like this, so expect more.

- [ ] **Pre-school booster missing.** The render example in [spec/rust-impl.md](./rust-impl.md) lists a "4-in-1 pre-school booster" (DTaP/IPV) at 3y4m, but no such series exists in `schedules/uk-2026-01-01.toml` — only MMR dose 2. Add the series, the `4-in-1` product class, and the pre-school booster product(s).

## Deferred (designed for, explicitly not v1)

These are out of scope now but the format and engine must not preclude them — that is the whole point of the v1 design discipline.

- **Historical versioning (v2).** `load_schedule_for_date(dir, country, date)` selecting the schedule where `valid_from <= dob` with no nearer successor. Then curate ~8–12 historical UK versions back to ~1990. See [spec/standard.md](./standard.md) §"Historical Versioning" and the [README](../README.md).
- **At-risk / overriding rules.** DNS-MX-style numerical priority on eligibility rules; higher-priority matching rule overrides the primary schedule. [spec/standard.md](./standard.md) §"Future extensions".
- **Catch-up schedules.** Distinct from primary-schedule evaluation; v1 only flags incomplete series.
- **Multi-jurisdiction.** Schedule files are currently flat (`schedules/uk-<date>.toml`); when a second jurisdiction is added this splits into per-country subdirectories (`schedules/<country>/<date>.toml`) without a format change, and `products/<coding-system>.toml` is already per-coding-system. No non-UK data yet.

## Suggested near-term order

~~M0 (commit + CI)~~ ✓ → ~~M1 (all design questions resolved)~~ ✓ → **next: M2** (eligibility enforcement, the resolved status model, out-of-schedule labelling, unmatched-dose reporting) → M4 `validate` then `render` (cheap, high-signal, and `render` proves the thesis) → backfill M5 fixtures alongside.
