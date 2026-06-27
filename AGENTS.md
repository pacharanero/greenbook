# AGENTS.md

## Repository Shape

greenbook is a proof-of-concept computable UK Green Book schedule plus evaluators.

- `spec/` is the language-neutral source of design truth. Start with `spec/standard.md`, `spec/conformance-vs-coverage.md`, and `spec/roadmap.md`.
- `rules/` holds canonical TOML sources: schedule versions (`schedule-*.toml`) and product maps (`product-map-*.toml`).
- `rust/` is the reference implementation and conformance-golden generator.
- `js/` is a peer JavaScript implementation, kept aligned to Rust through the shared conformance suite.
- `conformance/` contains FHIR fixtures, case metadata, and Rust-generated expected JSON outputs.
- `docs/demo/` is a static demo. `docs/demo/engine.js` and `docs/demo/data.js` are generated; edit `js/greenbook.js` or canonical sources, then regenerate.

## Current State

The core current-schedule evaluator is implemented in Rust and JS:

- FHIR R4 bundle parsing.
- TOML schedule and product-map loading.
- Product-class conformance matching, separate from antigen coverage.
- Eligibility checks, including sex/birth-cohort uncertainty flags.
- Up-to-date-for-age headline status plus strict `fully_vaccinated`.
- Out-of-schedule labelling.
- Unmatched-dose reporting.
- MMR-style allocation across several series sharing one product class.
- Dose-sequence cross-check flags.
- Duplicate echo detection by shared procedure code.

The Rust reference output does not yet include `by_antigen`; JS computes it for the demo only and drops it before conformance comparison.

## Common Commands

Run from the repository root unless noted.

```sh
cargo test --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cd js && npm test
node docs/demo/build-data.mjs
```

Regenerate Rust conformance goldens deliberately after intended engine-output changes:

```sh
cd rust && cargo run --bin conformance -- --generate
```

## Editing Notes

- Keep generated demo files in sync if `js/greenbook.js`, fixtures, schedule, or product map changes.
- Do not treat antigen overlap as schedule conformance; conformance is by `product_class`.
- Historical versioning is the next high-value feature. Before implementing it, resolve the open semantics in `spec/queries.md`, especially whether historical evaluation uses one DOB-selected schedule or valid-time rules per due date / administration date.
- Use the local `sct` SNOMED tooling when curating historical vaccine products. Prefer `sct lexical <brand-or-class>` to find candidate UK dm+d concepts, then `sct lookup <sctid>` to verify the concept identity before adding it to `rules/product-map-uk-snomed-dm.toml` or fixtures. Do not copy old/demo SCTIDs forward without verification; several stale-looking codes can resolve to unrelated current products or no concept at all. `sct` can also help with ReadV2/CTV3/SNOMED crosswalks if older source data arrives in primary-care coding systems.
