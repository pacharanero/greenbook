#!/usr/bin/env node
/**
 * Validate the JS engine port (engine.js) against the Rust engine, by running
 * the Rust CLI on each bundled fixture and deep-comparing the result with the
 * JS engine's output on the same data.
 *
 *   node docs/demo/validate.mjs
 *
 * Compares everything the Rust CLI emits (status, fully_vaccinated, every
 * series, every recorded dose, unmatched doses). The JS-only by_antigen
 * coverage view is excluded since Rust does not produce it.
 */
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, '..', '..');

// Load the browser globals (data.js, engine.js) into a fake window.
const window = {};
for (const f of ['data.js', 'engine.js']) {
  new Function('window', readFileSync(join(here, f), 'utf8'))(window);
}
const { GREENBOOK } = window;

function rustEval(fixtureId, evaluatedAt) {
  const out = execFileSync('cargo', [
    'run', '--quiet', '--bin', 'greenbook', '--',
    'evaluate',
    'schedules/uk-2026-01-01.toml', 'products/uk-snomed-dm.toml',
    `tests/fixtures/${fixtureId}.json`,
    '--evaluated-at', evaluatedAt, '--format', 'json',
  ], { cwd: repo, encoding: 'utf8' });
  return JSON.parse(out);
}

// Strip the JS-only field so the two are comparable.
function comparable(obj) {
  const { by_antigen, ...rest } = obj;
  return JSON.parse(JSON.stringify(rest));
}

let failures = 0;
for (const fx of GREENBOOK.fixtures) {
  const rust = rustEval(fx.id, fx.evaluatedAt);
  const js = comparable(window.Greenbook.evaluate(fx.record, GREENBOOK.schedule, GREENBOOK.products, fx.evaluatedAt));
  const a = JSON.stringify(rust);
  const b = JSON.stringify(js);
  if (a === b) {
    console.log(`  OK   ${fx.id}  (${rust.status}, fully_vaccinated=${rust.fully_vaccinated})`);
  } else {
    failures++;
    console.log(`  FAIL ${fx.id}`);
    // Show the first differing series for a quick diagnosis.
    for (let i = 0; i < rust.by_series.length; i++) {
      const r = JSON.stringify(rust.by_series[i]);
      const j = JSON.stringify(js.by_series[i]);
      if (r !== j) { console.log('    rust:', r); console.log('    js:  ', j); break; }
    }
    if (JSON.stringify(rust.unmatched_doses) !== JSON.stringify(js.unmatched_doses)) {
      console.log('    rust unmatched:', JSON.stringify(rust.unmatched_doses));
      console.log('    js   unmatched:', JSON.stringify(js.unmatched_doses));
    }
  }
}

console.log(failures === 0 ? '\nAll fixtures match the Rust engine.' : `\n${failures} fixture(s) diverged.`);
process.exit(failures === 0 ? 0 : 1);
