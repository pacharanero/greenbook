# Testing the POC walkthrough

A step-by-step run through the entire POC. Every command is run from the repo root.

## 0. Prerequisites

Verify the toolchain is installed:

```sh
cargo --version   # cargo 1.93+ tested
rustc --version   # rustc 1.93+ tested
git --version
curl --version    # for the PDF download script
```

If any are missing, install Rust via [rustup](https://rustup.rs/).

## 1. Project layout

```
greenbook/
  Cargo.toml                 - crate manifest
  src/
    lib.rs                   - public re-exports
    age.rs                   - AgeOffset parser ("8 weeks", "3 years 4 months")
    schedule.rs              - schedule.toml deserialiser
    products.rs              - product mapping deserialiser
    fhir.rs                  - FHIR R4 Bundle parser (Patient + Immunization)
    evaluate.rs              - the evaluation engine
    error.rs                 - error types
    bin/greenbook.rs         - CLI entry point
  schedules/
    uk-2026-01-01.toml       - the current UK schedule (lifted from spec/standard.md)
  products/
    uk-snomed-dm.toml        - SNOMED UK drug extension product → antigens map
  tests/
    evaluate.rs              - integration test
    fixtures/
      six-month-fully-vaccinated.json
  s/
    download-green-book.sh   - utility to fetch source PDFs from gov.uk
  pdf/
    green-book-chapter-11-2026-03-30.pdf   - downloaded source PDF (gitignored)
  spec/                      - the specification documents (incl. roadmap)
```

## 2. Fetch the source Green Book PDF (optional)

The schedule TOML is the source of truth for the evaluator, but if you want the original PDF on disk for reference:

```sh
s/download-green-book.sh
```

It is idempotent — re-runs skip files already present. Pass `--force` to re-download.

## 3. Build the crate

```sh
cargo build
```

First build pulls dependencies (chrono, serde, toml, serde_json, clap, thiserror) from crates.io and takes ~5s on a warm machine. Subsequent builds are incremental.

A clippy-clean build:

```sh
cargo clippy --all-targets -- -D warnings
```

## 4. Run the test suite

```sh
cargo test
```

Expected:

- 3 unit tests in `src/age.rs` (AgeOffset parsing and date arithmetic)
- 1 integration test in `tests/evaluate.rs` (`six_month_old_on_schedule_evaluates_correctly`)

Total: **4 passed**.

The integration test pins the evaluation date to **2026-04-29** so the result is deterministic regardless of when you run it.

## 5. Run the CLI on the bundled fixture

The fixture represents a 6-month-old female, DOB 2025-10-29, who has received every immunisation due so far at her 8-week, 12-week and 16-week visits, all from the latest Green Book products.

Human-readable report:

```sh
cargo run --quiet --bin greenbook -- \
  evaluate \
  schedules/uk-2026-01-01.toml \
  products/uk-snomed-dm.toml \
  tests/fixtures/six-month-fully-vaccinated.json \
  --evaluated-at 2026-04-29
```

JSON output:

```sh
cargo run --quiet --bin greenbook -- \
  evaluate \
  schedules/uk-2026-01-01.toml \
  products/uk-snomed-dm.toml \
  tests/fixtures/six-month-fully-vaccinated.json \
  --evaluated-at 2026-04-29 \
  --format json
```

If you omit `--evaluated-at` it defaults to today; the same fixture will give different results once you cross the next due-date boundary.

## 6. What you should see

The README [Walkthrough](../README.md#walkthrough) is the canonical, annotated tour of the output across several demonstration fixtures. In brief, for this on-schedule infant the salient lines are:

- `Up-to-date status: UP_TO_DATE_FOR_AGE` — the headline, age-relative answer: every dose due so far has been given. Distinct from `Fully vaccinated: no`, the strict "every dose at every age" flag, which is correctly `no` for a 6-month-old.
- `[COMPLETE   ] 6-in-1 (3/3 due, 3 total) - up to date` — three valid doses, all on or after `earliest_age`.
- `[PARTIAL    ] MenB (2/2 due, 3 total) - up to date` — dose 3 isn't due until 12 months, so the patient is still up to date despite the series being incomplete.
- `[NONE       ] MMR (first dose) (0/0 due, 1 total) - up to date` — nothing due yet.

The booster series (Hib/MenC, Td/IPV) show no doses and no spurious `OUT-OF-SCHEDULE` entries: conformance matches doses to series by **product class**, so a 6-in-1 dose is never dragged into a booster series via shared antigens. See [ADR 0001](adr/0001-product-class-conformance-vs-antigen-coverage.md) for the conformance-vs-coverage decision behind this.

## 7. Try changing the inputs

Quick experiments to confirm the engine is doing real work, not pattern matching. The other bundled fixtures (`behind-for-age-toddler.json`, `out-of-schedule-doses.json`, `unmatched-doses.json`) already show these effects; you can also edit `six-month-fully-vaccinated.json` yourself:

| Change | Expected effect |
|---|---|
| Edit the fixture's `birthDate` to 2024-04-29 (2-year-old) | MMR-primary, Hib/MenC, MenB dose 3, PCV dose 2 are now due but missing, so they show as `BEHIND` and the headline becomes `BEHIND_FOR_AGE`. |
| Edit a dose date so 6-in-1 dose 2 is given on 2025-12-29 (only 5 days after dose 1) | That dose is flagged `OUT-OF-SCHEDULE` with reason "interval from previous dose < 4 weeks", and does not count toward completion. |
| Delete the rotavirus dose 1 entry from the fixture | `rotavirus-primary` flips from `Complete` to `Partial` (1/2 doses). |
| Change a vaccineCode to a SNOMED code not in `products/uk-snomed-dm.toml` | The dose appears in the `Unmatched doses` section as an "unknown product code" rather than disappearing silently. |

## 8. Inspecting the schedule itself

Open `schedules/uk-2026-01-01.toml` directly. Every series is one `[[series]]` block with its doses inline; antigen IDs at the bottom map to SNOMED concept codes. Editing this file (and re-running step 5) is how you would propose a schedule change.

## 9. Next steps

The design questions the POC raised are now resolved and folded into the [spec](../spec/) and [roadmap](../spec/roadmap.md) (see the roadmap's M1 for the record). Most of M2 is now implemented (eligibility enforcement, the up-to-date-for-age status model, out-of-schedule labelling, and unmatched-dose reporting). The next steps are the dose-sequence cross-check (M2) and the other CLI commands - `validate` and `render` (M4).
