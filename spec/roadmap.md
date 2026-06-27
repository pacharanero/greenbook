# Roadmap

Where greenbook is and where it goes next. This tracks the gap between the [specification](./) and the implementation, and sequences the remaining work. It is a living document - edit it as decisions land. The design questions raised during the POC are resolved and recorded in M1 below; new open questions are captured in [queries.md](./queries.md).

## Status at a glance

The v1 POC core is working and covered by the shared conformance suite. The engine parses a FHIR R4 bundle, loads a TOML schedule and product map, and produces the up-to-date-for-age status, per-series breakdown, out-of-schedule flags, unmatched-dose list, duplicate-dose list, and soft dose-sequence flags as a human-readable, status-only, or JSON report. One schedule version (`schedule-uk-2026-01-01.toml`), one product map, and seven conformance fixtures are bundled (see the README [Walkthrough](../README.md#walkthrough)).

What this means: the file format and current-schedule evaluation pipeline are proven end-to-end, and the M1 decisions are implemented in the engine. The remaining work is historical versioning, reference-output antigen coverage, additional CLI commands, schedule-content gaps, and broader test coverage - none of it requires rearchitecting, but historical versioning now needs the most design attention.

The baseline is committed and CI is green (M0 done), the POC design questions are resolved (M1), and M2 is implemented. The next priority is Historical Versioning, because it is the project's highest-value differentiator: retrospectively determining whether a person was correctly vaccinated for their age is not tractable by hand across decades of UK schedule changes, but it becomes deterministic once the schedule history is computable.

The repository is now organised as **peer implementations**: [`rust/`](../rust/) (the reference) and [`js/`](../js/), with canonical sources/spec at the top level and a shared [`conformance/`](../conformance/) suite (Rust-generated golden outputs that every implementation is tested against). New languages (Ruby, Python) join by running the same suite. A richer per-language docs site (e.g. Zensical) with shared/common sections is planned.

## Guiding principle

Build the format and engine correctly for the *current* schedule so that historical versioning, at-risk overrides, and catch-up rules become additive extensions rather than rewrites. Every design choice made now is judged against that test. See the [README](../README.md) §"Scope for v1".

## Milestones

### M0 — Baseline commit and CI

Get the working POC onto a reviewable footing before adding behaviour.

- [x] Initial git commit of the current tree
- [x] `CHANGELOG.md` (referenced by the schedule `change_summary` but absent)
- [x] CI workflow running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` (per [spec/rust-impl.md](./rust-impl.md) §"Build and Tooling Conventions"; confirm latest stable action versions before pinning)

### M1 — Resolve the open design questions

These were decisions, not code — each needed a ruling before the dependent code was worth writing. All are now resolved; the rulings are recorded below and folded into the spec.

- [x] §1 Dose-to-series matching — **resolved**: product-class conformance matching vs antigen coverage, [conformance vs coverage](./conformance-vs-coverage.md). Implemented.
- [x] §3 Suppressing irrelevant invalid doses — **resolved**: falls out of §1; no suppression needed.
- [x] §2 "Fully vaccinated" vs "up-to-date for age" — **resolved**: `UpToDateForAge` (`BehindForAge`/`Unvaccinated`/`Unknown`) is the headline status; a strict `fully_vaccinated` flag is retained alongside. "Fully immunised for life stage" rejected as a moving target. See [spec/standard.md](./standard.md) §"Overall status".
- [x] §5 `latest_age` semantics for late-but-given doses — **resolved**: a dose breaking an age/interval rule (too early *or* too late) is recorded as received but labelled "outside standard schedule" and does not count toward completion. See [spec/standard.md](./standard.md) §"Dose validity".
- [x] §4 HPV sex restriction — settled in the spec (gender `other`/`unknown` → eligible + uncertainty flag). Implemented.
- [x] §6 crate/binary name — `greenbook` for both (verified free on crates.io 2026-06-05).
- [x] §7 schedule directory layout — flat `rules/schedule-uk-<YYYY-MM-DD>.toml` (schedules and product maps share `rules/`, distinguished by filename prefix); jurisdiction code `UK`. See [spec/standard.md](./standard.md) §"Directory structure".

### M2 — Correctness gaps in the engine

The known divergences from [spec/standard.md](./standard.md) §"Evaluation Logic" are closed.

- [x] Product-class conformance matching ([conformance vs coverage](./conformance-vs-coverage.md)) — 6-in-1 doses no longer falsely flagged under booster series.
- [x] **Enforce eligibility.** `population` and `male_born_on_or_after` are now checked: ineligible series are `NotApplicable` and excluded from the overall status, and a `gender = other|unknown` patient on a sex-restricted series is treated as eligible with an `eligibility_uncertain` flag (M1 §4).
- [x] **Adopt the resolved status model (§2).** `OverallStatus` is now the headline age-relative enum `UpToDateForAge`/`BehindForAge`/`Unvaccinated`/`Unknown`, `VaccinationStatus` carries the strict `fully_vaccinated: bool`, and each series carries `doses_due` and `up_to_date_for_age`. See [spec/standard.md](./standard.md) §"Overall status".
- [x] **Out-of-schedule labelling (§5).** `RecordedDose` now uses `within_schedule`/`schedule_notes`; rule-breaking doses (too early or too late) are reported as "outside standard schedule" rather than "invalid".
- [x] **Unmatched-dose reporting.** `VaccinationStatus.unmatched_doses` surfaces both an *unknown* product code and a *known* product whose class matches no series in the loaded schedule (e.g. 5-in-1 vs the 2026 schedule).
- [x] **One product class shared by several series — dose allocation.** *Discovered via the web demo (a child with both MMR doses).* A class that maps to several series (`MMR` → `mmr-primary` + `mmr-second`) is now evaluated as one **programme**: the class's doses are allocated across the series' slots in date order, with interval-from-previous spanning the programme. No more spurious "extra dose" / "too early" flags for an on-time both-MMR record. Fixed in the Rust engine and the JS port, re-validated, with a `mmr-both-doses` fixture. See [spec/standard.md](./standard.md) §"One product class, several series".
- [x] **Dose-sequence cross-check.** Date order is authoritative for allocation; the recorded `protocolApplied.doseNumberPositiveInt` and the SNOMED procedure code (UKCore-VaccinationProcedure extension) are cross-checks that raise a soft `flag` on the `RecordedDose` rather than overriding ([spec/standard.md](./standard.md) §"Dose sequencing"). The FHIR parser now reads the procedure code/display.
- [x] **Duplicate ("echo") detection.** The same physical jab recorded twice from different systems (different dates, same procedure code) is collapsed: earliest kept, the rest reported in `duplicate_doses` ([spec/standard.md](./standard.md) §"Duplicate doses"). Fixture: `duplicate-echo`.

### M3 — Historical Versioning

This is now the priority work. The core product claim is not just "evaluate the current schedule" but "evaluate whether a person was correctly vaccinated for their age at a point in time, using the schedule that actually applied to them." That is intractable for a human across decades of UK schedule changes and becomes deterministic once the Green Book history is computable. SystmOne Online has been observed taking the naive failure-mode approach - projecting a current-schedule grid onto a historical patient record - which makes this more than a theoretical risk.

The valid-time decision is recorded in [queries.md](./queries.md): historical evaluation is by dose due date, not by a single DOB-selected snapshot.

- [x] Decide the historical selection rule: dose slots are selected by the schedule version in force when each dose first became due; future not-yet-due slots are projected from the version in force on `evaluated_at`.
- [x] Implement schedule-version discovery over `rules/schedule-<country>-*.toml`, optional `valid_to`, derived effective ranges, gap/overlap errors, and effective-schedule construction with tests.
- [x] Add `evaluate-auto <rules-dir> <product-map> <bundle>` so the caller can select schedules automatically rather than passing one schedule file.
- [x] Add `versions <rules-dir> [--country <code>]` so historical schedule files are inspectable from the CLI.
- [x] Discover and cache the initial historical source corpus: the 2006 whole-book Green Book and GOV.UK-era Chapter 11 PDFs from 2013-2026, recorded in [`sources/green-book-schedule-sources.toml`](../sources/green-book-schedule-sources.toml) and fetched by [`s/download-green-book.sh`](../s/download-green-book.sh).
- [x] Curate the first non-current historical schedule version: [`rules/schedule-uk-2006-11-01.toml`](../rules/schedule-uk-2006-11-01.toml), derived from the 2006 Green Book whole-book PDF, covering the 5-in-1 primary course and bounded before the December 2010 Chapter 11 snapshot.
- [x] Curate the December 2010 schedule slice: [`rules/schedule-uk-2010-12-01.toml`](../rules/schedule-uk-2010-12-01.toml), adding the girls-only three-dose HPV programme while retaining the 5-in-1 infant course.
- [x] Curate the September 2014 schedule slice: [`rules/schedule-uk-2014-09-01.toml`](../rules/schedule-uk-2014-09-01.toml), adding rotavirus, the teenage MenC booster, and the two-dose HPV schedule.
- [x] Add a targeted historical regression test showing a Pediacel 5-in-1 dose conforms under the 2006 schedule but is unmatched under the 2026 schedule.
- [ ] Add a historical conformance fixture where a Pediacel 5-in-1 dose conforms under its historical schedule but is not silently treated as a 6-in-1 dose.
- [ ] Find the 1992 and 1996 printed Green Book editions, plus any predecessor childhood vaccination schedule documents, through targeted archival/library search.
- [ ] Build out the historical UK schedule set in small, reviewed slices (~8-12 versions back to roughly 1990 unless the source audit says otherwise).

### M4 — Output completeness

Bring the output up to the specced shape.

- [ ] **Reference antigen-coverage view** (`by_antigen` / `AntigenStatus`) — the "what diseases is this child protected against?" computation, deliberately deferred when [conformance vs coverage](./conformance-vs-coverage.md) split conformance from coverage. JS computes this for the demo today; Rust/reference output and conformance goldens do not yet include it.
- [ ] Reconcile types with the spec where they have drifted (e.g. `AgeOffset` vs a separate `Interval`; whether `AgeOffset` needs `Ord` or a dedicated sort key - `render` will require sorting by age).
- [ ] **Predicted future schedule** (consumer need 3 from M1 §2). Project the doses a patient has not yet reached the age for, as a forward-looking "what's next and when" list (assuming no schedule change). Useful to parents and planners.
- [ ] **Broader record-error detection** (consumer need 4 from M1 §2). Duplicate echoes are already detected; remaining record-quality checks include implausibly spaced administrations beyond schedule-window checks, doses before birth, and other record errors distinct from schedule non-conformance.

### M5 — The rest of the CLI

Only `evaluate` exists (`report`, `json`, and `status` output). The other commands are specced in [spec/rust-impl.md](./rust-impl.md) §"CLI"; `versions` is tracked under Historical Versioning because it directly supports curation and schedule selection.

- [ ] `validate <schedule>` — structural + referential + logical-consistency checks (sequential dose numbers, interval present where dose > 1). Natural next command; reuses existing validation.
- [ ] `render <schedule> [--format table|markdown|html]` — series-centric → age-centric pivot. **This is the command that demonstrates the core thesis: the publication PDF generated from the data, not the reverse.**
- [ ] `diff <a> <b> [--format table|json]` — compare two schedule versions for PR review.

### M6 — Test coverage

Seven conformance fixtures are now bundled and covered by integration tests: `six-month-fully-vaccinated`, `behind-for-age-toddler`, `out-of-schedule-doses`, `unmatched-doses`, `mmr-both-doses`, `duplicate-echo`, and `dose-number-mismatch`. Remaining fixtures from the spec's list - several deliberately exercise structures that historical versioning and catch-up features will lean on:

- [ ] `partial_hpv` and an explicit `unvaccinated` older child
- [ ] `sex_unknown_hpv` — exercises the M2 `eligibility_uncertain` flag
- [ ] `product_5in1_to_6in1` — historical Pediacel dose vs current 6-in-1 schedule
- [ ] `product_mmrv_to_mmr` — lossy substitution in the opposite direction; **requires adding the MMRV product and a `varicella` antigen** to the registry/map ([spec/standard.md](./standard.md) §"Product Mapping File")
- [ ] `catch_up_age_3` — late presenter; exercises the eligibility structure against the catch-up case ahead of M-future
- [ ] Fold the [test-data/](../test-data/) MMR catch-up scenarios into integration tests

### Demonstration and documentation

* [x] Reveal.js presentation of the entire 'thought chain' of the project - explaining to a general clinical/technical audience the chain from the current Green Book state of affairs through the concepts of schedule, products, antigens, and coverage/conformance, using our ubiquitous language throughout. Built with the revealjs skill: [docs/presentation/](../docs/presentation/).
* [x] Web-based demo ([docs/demo/](../docs/demo/)). Dashboard and timeline views are static-site ready and run on the JavaScript port of the engine. Presets cover the conformance fixtures, the demo shows recorded doses, product classes, antigens, conformance by series, antigen coverage, the headline status, duplicate echoes, and sequence flags. The custom patient builder is implemented: set DOB/evaluation date/sex, select scheduled doses, add off-schedule or unknown doses, and see the report update live.

### M7 — Schedule content gaps

Gaps in the *data* (not the engine) found under programmatic scrutiny. The Green Book has never been machine-checked like this, so expect more.

- [ ] **Pre-school booster missing.** The render example in [spec/rust-impl.md](./rust-impl.md) lists a "4-in-1 pre-school booster" (DTaP/IPV) at 3y4m, but no such series exists in `rules/schedule-uk-2026-01-01.toml` — only MMR dose 2. Add the series, the `4-in-1` product class, and the pre-school booster product(s).

## Deferred

These are still out of scope for the immediate historical-versioning push, but the format and engine must not preclude them.

- **At-risk / overriding rules.** DNS-MX-style numerical priority on eligibility rules; higher-priority matching rule overrides the primary schedule. [spec/standard.md](./standard.md) §"Future extensions".
- **Catch-up schedules.** Distinct from primary-schedule evaluation; v1 only flags incomplete series.
- **Multi-jurisdiction.** Schedules and product maps are currently flat in `rules/` (`schedule-uk-<date>.toml`, `product-map-uk-<coding-system>.toml`); the jurisdiction code is already in each filename, so a second jurisdiction is added by dropping its files alongside, with no format change. Per-country subdirectories remain an option if a single jurisdiction's history grows large. No non-UK data yet.

## Suggested near-term order

~~M0 (commit + CI)~~ -> ~~M1 (design questions resolved)~~ -> ~~M2 (current-schedule correctness)~~ -> **M3 Historical Versioning in progress** (next: curate first historical Green Book version and fixture) -> M5 `validate`/`diff` to support curation -> M4 reference antigen coverage and predicted-next output -> M5 `render` -> backfill M6 fixtures alongside.
