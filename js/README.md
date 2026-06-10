# greenbook - JavaScript implementation

A JavaScript implementation of the greenbook evaluation engine: parse a patient's FHIR vaccination history and evaluate it against a [computable schedule](../schedules/). Runs in the browser (it powers the [demo](../docs/demo/)) and in Node.

[Rust](../rust/) is the reference implementation; this one is kept in step by the shared [conformance suite](../conformance/). The two are independent ports of the same [specification](../spec/).

## Use it

`greenbook.js` is a single UMD file with no runtime dependencies - load it as a browser global or import it in Node.

```js
// Node (CommonJS / ESM interop)
import gb from './greenbook.js';        // or: const gb = require('./greenbook.js')

const record = gb.parseFhirBundle(fhirBundleJsonString);   // -> { dob, gender, immunisations[] }
const status = gb.evaluate(record, schedule, productMap, '2026-04-29');
```

```html
<!-- Browser: sets the global `Greenbook` -->
<script src="greenbook.js"></script>
<script> const status = Greenbook.evaluate(record, schedule, productMap, evalDate); </script>
```

`schedule` and `productMap` are the parsed [TOML sources](../schedules/) (objects, not TOML text). The result shape matches the reference engine, plus a JS-only `by_antigen` coverage view used by the demo.

## Test

```sh
node test/conformance.mjs      # or: npm test
```

Runs the engine over every case in [`../conformance/cases.json`](../conformance/) and asserts it reproduces the committed golden outputs. It needs only **Node and `python3`** (to read the canonical TOML, as the demo build does) - the Rust toolchain is not required, so this validates the JS implementation on its own.

## Layout

- `greenbook.js` - the engine (parsing, eligibility, conformance matching, dose-sequence allocation, duplicate detection, coverage). Mirrors `rust/src/`.
- `test/conformance.mjs` - the shared-conformance runner.
