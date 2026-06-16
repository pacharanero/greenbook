# Getting started

greenbook has two peer implementations of the same [specification](https://github.com/pacharanero/greenbook/tree/main/spec): a **reference** implementation in **Rust** (with a CLI) and a peer implementation in **JavaScript** (a dependency-free module that also powers the [demo](demo/index.html)). Both are kept in step by one shared [conformance suite](https://github.com/pacharanero/greenbook/tree/main/conformance).

There are no published binaries yet - this is a proof-of-concept, so you run it from a clone:

```sh
git clone https://github.com/pacharanero/greenbook.git
cd greenbook
```

All commands below run **from the repo root**, so the canonical data paths (`rules/…`, `conformance/…`) resolve.

## Evaluate a patient

=== ":material-language-rust: Rust"

    With a [Rust toolchain](https://rustup.rs) (stable):

    ```sh
    cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- \
      evaluate \
      rules/schedule-uk-2026-01-01.toml \
      rules/product-map-uk-snomed-dm.toml \
      conformance/fixtures/six-month-fully-vaccinated.json \
      --evaluated-at 2026-04-29
    ```

    `evaluate <schedule> <product-map> <bundle>` takes an optional `--evaluated-at YYYY-MM-DD` (defaults to today) and `--format report|json|status`. The default `report` prints the full per-series breakdown.

=== ":material-language-javascript: JavaScript"

    With [Node](https://nodejs.org) (and `python3`, used to read the TOML sources, exactly as the demo build does). `greenbook.js` is a single UMD file with **no runtime dependencies**:

    ```js
    import gb from './js/greenbook.js';     // or: const gb = require('./js/greenbook.js')

    const record = gb.parseFhirBundle(fhirBundleJsonString);   // -> { dob, gender, immunisations[] }
    const status = gb.evaluate(record, schedule, productMap, '2026-04-29');

    console.log(status.status);             // "up_to_date_for_age"
    ```

    `schedule` and `productMap` are the parsed TOML sources (objects, not text). The result shape matches the reference engine, plus a JS-only `by_antigen` coverage view. The easiest way to *see* the JS engine running is the [interactive demo](demo/index.html), which is this exact module in the browser.

## The headline answer, green or red

Most of the time you do not want the JSON - you want the one-line verdict. The `status` format **filters everything else out** and prints a single coloured line: green for up to date, red for not.

=== ":material-language-rust: Rust"

    ```sh
    cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- \
      evaluate \
      rules/schedule-uk-2026-01-01.toml \
      rules/product-map-uk-snomed-dm.toml \
      conformance/fixtures/six-month-fully-vaccinated.json \
      --evaluated-at 2026-04-29 \
      --format status
    ```

    <div class="result" markdown>
    <span style="color:#2e7d32;font-weight:700">Up to date for age</span>
    </div>

    Point it at a child who has missed due doses and you get the red verdict instead:

    ```sh
    # … conformance/fixtures/behind-for-age-toddler.json … --format status
    ```

    <div class="result" markdown>
    <span style="color:#c62828;font-weight:700">Not up to date for age</span>
    </div>

=== ":material-language-javascript: JavaScript / jq"

    If you would rather filter the JSON yourself - to script against it, or pull out one field - emit `--format json` and pipe through [`jq`](https://jqlang.github.io/jq/):

    ```sh
    cargo run --manifest-path rust/Cargo.toml --quiet --bin greenbook -- \
      evaluate \
      rules/schedule-uk-2026-01-01.toml \
      rules/product-map-uk-snomed-dm.toml \
      conformance/fixtures/six-month-fully-vaccinated.json \
      --evaluated-at 2026-04-29 \
      --format json | jq -r '.status'
    ```

    ```
    up_to_date_for_age
    ```

    The same `.status` field is what the JS engine returns as `status.status`.

!!! note "Colour is terminal-only"
    `--format status` emits ANSI colour only when stdout is a terminal and `NO_COLOR` is unset, so piping into a file or another command gives clean, plain text (`Up to date for age`).

Now that you can get the headline, the [walkthrough](walkthrough.md) drills past it - into which series conform, which diseases are covered, and how the engine handles the messy cases - using the bundled conformance fixtures.
