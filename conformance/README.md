# Conformance suite

The shared test harness that keeps every implementation of greenbook in step. Each implementation (currently [Rust](../rust/) and [JavaScript](../js/)) runs the **same** cases against the **same** canonical sources and asserts it reproduces the **same** golden outputs.

## Contents

- `cases.json` - the case manifest. Each case names a `fixture`, the `schedule` and `products` (paths relative to the repo root), and the `evaluated_at` date.
- `fixtures/*.json` - the FHIR R4 bundles (the patients).
- `expected/<id>.json` - the golden evaluation output for each case.

## How it works

[Rust](../rust/) is the reference implementation and **generates** the goldens:

```sh
cd rust && cargo run --bin conformance -- --generate
```

Every implementation then has a test that, for each case, loads the canonical schedule + product map ([`../schedules/`](../schedules/), [`../products/`](../products/)) and the fixture, evaluates, and asserts deep (key-order-independent) equality with the golden:

- Rust - `rust/tests/conformance.rs` (run by `cargo test`)
- JavaScript - `js/test/conformance.mjs` (run by `node test/conformance.mjs`); validates standalone, without the Rust toolchain.

A new implementation (Ruby, Python, ...) joins by adding its own runner over `cases.json` + `expected/`.

## Notes

- The goldens are the reference engine's output. They exclude the JS-only `by_antigen` coverage view (the reference engine does not compute coverage yet); the JS runner drops that field before comparing.
- When engine behaviour changes intentionally, regenerate the goldens and review the `expected/` diff in the same change.
