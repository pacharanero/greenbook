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
  spec/                      - the specification documents
  queries.md                 - open questions raised during POC build
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

For the human-readable report, the salient lines are:

- `Overall status: PARTIALLY_VACCINATED` — technically correct but clinically misleading for an on-schedule infant. This is why the status model is being reworked so the headline answer is **up-to-date for age** (see [queries.md](../queries.md) §2, resolved); the engine change is tracked on the [roadmap](../spec/roadmap.md) (M2). The current code still reports the strict status.
- `[COMPLETE   ] 6-in-1 (3/3 doses)` — three valid doses, all on or after `earliest_age`.
- `[COMPLETE   ] Rotavirus (2/2 doses)` — both doses given before the 14w 6d / 23w 6d cutoffs.
- `[PARTIAL    ] MenB (2/3 doses)` — third dose isn't due until 12 months.
- `[PARTIAL    ] PCV (1/2 doses)` — second dose isn't due until 12 months.
- `[NONE       ] Hib/MenC booster, MMR (×2), HPV, Td/IPV` — all due later than 6 months.

The booster series (Hib/MenC, Td/IPV) show no doses and no spurious `INVALID` entries: conformance now matches doses to series by **product class**, so a 6-in-1 dose is never dragged into a booster series via shared antigens. This resolved [queries.md](../queries.md) §1 — see [ADR 0001](adr/0001-product-class-conformance-vs-antigen-coverage.md).

## 7. Try changing the inputs

Quick experiments to confirm the engine is doing real work, not pattern matching:

| Change | Expected effect |
|---|---|
| Edit the fixture's `birthDate` to 2024-04-29 (2-year-old) | MMR-primary, Hib/MenC, MenB dose 3, PCV dose 2 should now show as `NONE` because they were due but not given. Overall stays `PARTIALLY_VACCINATED`. |
| Edit a dose date so 6-in-1 dose 2 is given on 2025-12-29 (only 5 days after dose 1) | That dose should be flagged `INVALID` with reason "interval from previous dose < 4 weeks". |
| Delete the rotavirus dose 1 entry from the fixture | `rotavirus-primary` flips from `Complete` to `Partial` (1/2 doses). |
| Change a vaccineCode to a SNOMED code not in `products/uk-snomed-dm.toml` | The dose silently disappears from the evaluation — no series matches it. (Worth adding an "unknown product code" warning in a follow-up.) |

## 8. Inspecting the schedule itself

Open `schedules/uk-2026-01-01.toml` directly. Every series is one `[[series]]` block with its doses inline; antigen IDs at the bottom map to SNOMED concept codes. Editing this file (and re-running step 5) is how you would propose a schedule change.

## 9. Next steps

See [queries.md](../queries.md) for the design questions the POC raised; most are now resolved and folded into the [spec](../spec/) and [roadmap](../spec/roadmap.md). The next implementation work is M2 (eligibility enforcement, the up-to-date-for-age status, and unmatched-dose reporting).
