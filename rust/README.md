# greenbook - Rust implementation

The **reference** implementation of the greenbook evaluation engine, and its CLI. Parses a FHIR R4 vaccination bundle and evaluates it against a [computable schedule](../schedules/).

This is one of several implementations kept in step by the shared [conformance suite](../conformance/); it is the one that **generates** the conformance goldens. See the [specification](../spec/) for the language-neutral design.

## Build & test

```sh
cargo test                                   # unit + integration + conformance
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## CLI

```sh
# from the repo root, so the data paths below resolve:
cargo run --manifest-path rust/Cargo.toml --bin greenbook -- \
  evaluate schedules/uk-2026-01-01.toml products/uk-snomed-dm.toml \
  ../conformance/fixtures/six-month-fully-vaccinated.json --evaluated-at 2026-04-29
```

`evaluate <schedule> <product-map> <bundle> [--evaluated-at YYYY-MM-DD] [--format report|json]`. (Other commands - `validate`, `render`, `versions`, `diff` - are specced in [spec/rust-impl.md](../spec/rust-impl.md) and not yet built.)

## Conformance goldens

Rust is the reference, so it writes the golden outputs every implementation tests against:

```sh
cargo run --bin conformance -- --generate     # rewrite conformance/expected/
cargo run --bin conformance                   # check current output matches (no flag)
```

`cargo test` also checks the goldens (`tests/conformance.rs`). Regenerate deliberately when behaviour changes, and review the `conformance/expected/` diff.

## Layout

- `src/evaluate.rs` - the engine (eligibility, conformance matching, dose-sequence allocation across a shared product class, duplicate detection).
- `src/fhir.rs` - FHIR bundle parser; `src/schedule.rs` / `src/products.rs` - TOML loaders; `src/age.rs` - age arithmetic.
- `src/bin/greenbook.rs` - the CLI; `src/bin/conformance.rs` - the golden generator.
- `tests/` - integration tests + the shared conformance check.
