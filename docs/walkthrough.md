# Walkthrough

The [headline answer](getting-started.md) is the start, not the end. This walkthrough drills past it using the bundled **conformance fixtures** - the same FHIR bundles every implementation is tested against. Each one is a small, deliberate scenario.

Every command runs from the repo root and uses the Rust CLI; the JavaScript engine and the [interactive demo](demo/index.html) produce the same results.

| Fixture | Demonstrates |
|---------|--------------|
| `six-month-fully-vaccinated` | An on-schedule infant: up to date for age, not yet fully vaccinated |
| `behind-for-age-toddler` | Primary doses given, 12-month visit missed → behind for age |
| `out-of-schedule-doses` | Doses too early / too late → labelled, not counted |
| `unmatched-doses` | A 5-in-1 and an unknown code → surfaced, not dropped |
| `mmr-both-doses` | One product class, two series → allocated correctly |
| `duplicate-echo` | Same jab recorded twice → duplicate detected |
| `dose-number-mismatch` | Recorded dose number wrong → date order wins, flagged |

## Conformance: which series are satisfied

Drop `--format status` and the default `report` shows conformance **per series** - the heart of the engine. For the on-schedule 6-month-old:

```sh
cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- \
  evaluate rules/schedule-uk-2026-01-01.toml rules/product-map-uk-snomed-dm.toml \
  conformance/fixtures/six-month-fully-vaccinated.json --evaluated-at 2026-04-29
```

```text
Up-to-date status: UP_TO_DATE_FOR_AGE
Fully vaccinated:  no (strict: every eligible series complete)

By series:
---------
  [COMPLETE   ] 6-in-1 (3/3 due, 3 total) - up to date
      - ok              2025-12-24  dose 1  (1mo 3w 4d)  [Infanrix Hexa vaccine (product)]
      - ok              2026-01-21  dose 2  (2mo 3w 2d)  [Infanrix Hexa vaccine (product)]
      - ok              2026-02-18  dose 3  (3mo 2w 6d)  [Infanrix Hexa vaccine (product)]
  [PARTIAL    ] MenB (2/2 due, 3 total) - up to date
```

Read the counts as **valid / due (total)**. MenB is `PARTIAL` (2 of 3 doses) yet still "up to date" - its third dose is not due until 12 months, so it is not held against a 6-month-old. The booster series (Hib/MenC, Td/IPV) show **no doses and no spurious flags**, because conformance matches by [product class](concepts/conformance-vs-coverage.md): a 6-in-1 dose is never dragged into a booster series through shared antigens.

## Behind for age

Take a toddler with the same infant doses but who missed every 12-month appointment:

```sh
# … conformance/fixtures/behind-for-age-toddler.json …
```

```text
Up-to-date status: BEHIND_FOR_AGE
```

The infant series are still `COMPLETE`, but the series due at 12 months are now overdue, so the headline flips to `BEHIND_FOR_AGE`. The per-series breakdown is the catch-up worklist.

## Outside the standard schedule

`out-of-schedule-doses` has a 6-in-1 second dose given a week too soon and a rotavirus dose given after its hard cutoff. Those doses are **recorded but do not count**, and each carries a reason:

```text
      - OUT-OF-SCHEDULE 2025-12-31  dose 2  (...)  [Infanrix Hexa vaccine (product)]
          ! given before earliest_age ...
```

The engine does not call them "invalid" or hide them - they were real clinical events, [labelled honestly](concepts/status-model.md) as outside the standard schedule.

## Unmatched doses

`unmatched-doses` contains a Pediacel **5-in-1** (gone from the 2026 schedule) and an unknown code. Neither is silently dropped:

```text
Unmatched doses:
---------------
  - 2026-01-21  [Pediacel vaccine (product)]  (product class "5-in-1" has no series in this schedule version)
  - 2026-01-21  [Unknown investigational vaccine]  (unknown product code (not in the product map))
```

A 5-in-1 is unmatched **against today's schedule**; under [historical versioning](green-book/why-computable.md) it would conform to the schedule that was valid when it was given.

## One class, two series; and duplicate echoes

Two cases that trip up naive tools:

- **`mmr-both-doses`** - `MMR` is one product class serving *two* series (first and second dose). The engine allocates the recorded doses across the series' slots **by date order**, so both land correctly with no "extra dose" or "too early" flag.
- **`duplicate-echo`** - the same physical jab recorded twice by two systems. Detected by the shared SNOMED procedure code; the earliest is kept, the rest reported:

```text
Duplicate doses:
---------------
  - 2026-01-10  [Infanrix Hexa vaccine (product)]  (likely duplicate of 2025-12-24; same procedure code)
```

`dose-number-mismatch` is the related case: a dose recorded as "dose 2" that the dates prove is dose 1. **Date order is authoritative**; the recorded number is a cross-check that raises a soft flag rather than being trusted.

## Coverage: which diseases are protected

Conformance asks *"did they get the named doses?"*. **Coverage** asks the separate question *"which diseases is the child protected against?"*, computed across the [antigens](concepts/domain-model.md) of every product received. The reference engine reports conformance; the **JavaScript engine and the [interactive demo](demo/index.html) add a `by_antigen` coverage view**.

The demo is the best place to see both at once: its dashboard lays out recorded doses → conformance by series → antigen coverage → headline status, side by side, for every fixture above - or for a patient you build yourself.

[:octicons-arrow-right-24: Open the interactive demo](demo/index.html){ .md-button }
[:octicons-arrow-right-24: See the presentation](presentation/presentation.html){ .md-button }
