# greenbook demo

An interactive, dashboard-style demo of the evaluation engine. Pick a preset scenario (one of the bundled test fixtures) or **Build your own** patient - set the age and tick the doses given - and see the **layers of the logic** that produce the result:

1. **Recorded doses** decomposed into their **product class** and **antigens** (the product map);
2. **Conformance by series** - which doses count, matched by product class, with out-of-schedule and unmatched flags;
3. **Antigen coverage** - which diseases the child is protected against (the separate "coverage" question);
4. the headline **up-to-date-for-age** status and the strict **fully-vaccinated** flag.

It is intended to be shown after the [presentation](../presentation/): the presentation explains the ideas, the demo shows the engine running on real records.

## View it

Run `s/demo` from the repo root to serve the demo and open it in your browser, or just open `index.html` directly - no build step, no network. (To run the demo and the [presentation](../presentation/) together in one Docker container, use `s/up` instead - it serves both at http://localhost:8080/.) All data is embedded in `data.js`, so it also works unchanged from `file://`, a static server, or GitHub Pages.

## How it works

The demo is plain HTML/CSS/JS:

- `index.html` / `styles.css` - the dashboard shell and theme.
- `engine.js` - a **faithful JavaScript port of the Rust engine** (`src/evaluate.rs`). The Rust crate stays the source of truth; this port lets the demo evaluate entirely client-side, which is what makes live interactivity possible.
- `data.js` - the schedule, product map, and the demo patients, generated from the canonical files (see below).
- `app.js` - the dashboard wiring.

### Build your own (live editing)

The whole view is driven by one function, `renderScenario(record, evaluatedAt)`. Presets are one way to produce a `record`; the **Build your own** mode is another. It lets you set a date of birth, evaluation date and sex, tick the scheduled doses the child has had (only doses that are *due* by the evaluation date are selectable, so raising the age unlocks more of the schedule), and add off-schedule or unknown doses for edge cases. Each change rebuilds the `record` and re-runs the same pipeline, so everything below updates live.

The presets also exercise the dose-sequencing logic: **Both MMR doses** (one product class, two series - allocated correctly, no spurious flags), **Duplicate "echo" dose** (the same jab recorded twice with different dates but the same procedure code), and **Mis-keyed dose number** (recorded as dose 2 but it is dose 1 by date - flagged, not trusted).

## Regenerating the data

`data.js` is generated from the project's canonical data so it never drifts:

```sh
node docs/demo/build-data.mjs
```

This reads `schedules/uk-2026-01-01.toml`, `products/uk-snomed-dm.toml`, and `tests/fixtures/*.json` (TOML is converted with Python's stdlib `tomllib`; the FHIR bundles are reduced to the engine's record shape). Re-run it whenever the schedule, product map, or fixtures change.

## Validating the port

To confirm the JS engine matches the Rust engine, run both over every fixture and diff the output:

```sh
node docs/demo/validate.mjs
```

It runs the Rust CLI per fixture and deep-compares the result (status, every series, every recorded dose, unmatched doses) against `engine.js`. They currently match exactly.
