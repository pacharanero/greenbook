/**
 * greenbook demo - dashboard wiring.
 *
 * Data flow (designed for future interactivity):
 *
 *     record + evaluatedAt  ->  Greenbook.evaluate()  ->  render()
 *
 * Presets are just one way to produce a `record`. The whole view is driven by
 * renderScenario(record, evaluatedAt), so a future "Custom patient" mode only
 * needs to build a record from UI controls (DOB + selected doses) and call the
 * same function - nothing downstream changes.
 */
(function () {
  'use strict';

  const { schedule, products, fixtures } = window.GREENBOOK;
  const G = window.Greenbook;

  // Lookups.
  const seriesById = new Map(schedule.series.map((s) => [s.id, s]));
  const antigenById = new Map(schedule.antigen.map((a) => [a.id, a]));
  const productIndex = G.makeProductIndex(products);
  const scheduleClasses = new Set(schedule.series.map((s) => s.product_class));

  const esc = (s) => String(s == null ? '' : s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

  const OVERALL_LABEL = {
    up_to_date_for_age: 'Up to date for age',
    behind_for_age: 'Behind for age',
    unvaccinated: 'Unvaccinated',
    unknown: 'Unknown',
  };
  const SERIES_BADGE = {
    complete: ['ok', 'Complete'],
    partial: ['teal', 'Partial'],
    none: ['muted', 'None'],
    not_applicable: ['ghost', 'N/A'],
  };

  const antigenChip = (id, on) =>
    `<span class="chip ant ${on ? 'on' : 'off'}" title="${esc((antigenById.get(id) || {}).display_name || id)}">${esc(id)}</span>`;

  // A representative product for each schedule class, used by the custom builder
  // when a scheduled dose is ticked (e.g. class "6-in-1" -> Infanrix Hexa).
  const repProductByClass = new Map();
  for (const p of products.product) if (!repProductByClass.has(p.product_class)) repProductByClass.set(p.product_class, p);

  const shortName = (d) => String(d || '').replace(/\s+vaccine\s+\(product\)$/i, '');
  function abbreviateAge(str) {
    const o = G.parseAgeOffset(str);
    if (o.years) return o.years + 'y' + (o.months ? o.months + 'm' : '');
    if (o.months) return o.months + 'mo';
    if (o.weeks) return o.weeks + 'w';
    return o.days + 'd';
  }
  const AGE_PRESETS = [['new', 'Newborn'], ['w8', '8 weeks'], ['w16', '16 weeks'], ['m12', '1 year'], ['m18', '18 months'], ['m40', '3y 4m'], ['m144', '12 years'], ['m168', '14 years']];

  // --- State & selection ----------------------------------------------------

  let currentId = fixtures[0].id;

  // Custom-patient state. The whole view is record-driven, so this just feeds
  // buildCustomRecord() -> the same renderScenario pipeline as the presets.
  const CUSTOM = '__custom__';
  const todayISO = new Date().toISOString().slice(0, 10);
  const customState = {
    dob: G.fmtDate(G.addMonths(G.parseDate(todayISO), -18)), // ~18-month-old by default
    evaluatedAt: todayISO,
    gender: 'female',
    scheduleDoses: new Set(), // keys "<seriesId>#<doseNumber>" the child has had
    extraDoses: [],           // { code, display, date } off-schedule / unknown doses
  };

  function buildSidebar() {
    document.getElementById('presetList').innerHTML = fixtures
      .map((f) => `
        <button class="preset" data-id="${esc(f.id)}">
          <span class="preset-name">${esc(f.label)}</span>
          <span class="preset-hint">${esc(shortHint(f))}</span>
        </button>`)
      .join('');
    document.querySelectorAll('.preset[data-id]').forEach((btn) =>
      btn.addEventListener('click', () => { currentId = btn.dataset.id; render(); })
    );

    const j = schedule.jurisdiction;
    document.getElementById('sideFoot').innerHTML =
      `Schedule <span class="mono">${esc(schedule.schedule.valid_from)}</span><br>` +
      `${esc(j.country_name)} &middot; ${esc(j.schedule_authority)}<br>` +
      `Engine: JS port of the Rust core`;
  }

  // A short hint per preset (first clause of the description, roughly).
  function shortHint(f) {
    const HINTS = {
      'six-month-fully-vaccinated': 'Every dose due so far, given on time',
      'behind-for-age-toddler': 'Primary doses given, 12-month visit missed',
      'out-of-schedule-doses': 'Doses given too early / too late',
      'unmatched-doses': 'A 5-in-1 and an unknown code',
      'mmr-both-doses': 'One class, two series - allocated correctly',
      'duplicate-echo': 'Same jab recorded twice from two systems',
      'dose-number-mismatch': 'Recorded as dose 2, but it is dose 1',
    };
    return HINTS[f.id] || '';
  }

  // --- Render ---------------------------------------------------------------

  function render() {
    document.querySelectorAll('.preset[data-id]').forEach((b) =>
      b.classList.toggle('active', b.dataset.id === currentId)
    );
    if (currentId === CUSTOM) { renderCustom(); return; }
    document.getElementById('builder').innerHTML = ''; // hide the builder for presets
    const fx = fixtures.find((f) => f.id === currentId);
    renderScenario(fx.record, fx.evaluatedAt, fx);
  }

  // The single seam for the whole view. record-driven, not preset-driven.
  function renderScenario(record, evaluatedAt, meta) {
    const result = G.evaluate(record, schedule, products, evaluatedAt);
    renderTopbar(record, evaluatedAt, result, meta);
    renderKpis(record, result);
    renderPanels(record, result);
  }

  function renderTopbar(record, evaluatedAt, result, meta) {
    const age = G.ageBetween(G.parseDate(record.dob), G.parseDate(evaluatedAt));
    const sex = record.gender ? record.gender[0].toUpperCase() + record.gender.slice(1) : 'Unknown';
    const facts = [
      `<span class="fact">DOB <b class="mono">${esc(record.dob)}</b></span>`,
      `<span class="fact">Age at evaluation <b>${esc(age)}</b></span>`,
      `<span class="fact">Sex <b>${esc(sex)}</b></span>`,
      `<span class="fact">Evaluated <b class="mono">${esc(evaluatedAt)}</b></span>`,
      `<span class="fact">Schedule <b class="mono">${esc(result.schedule_version)}</b></span>`,
    ].join('');

    document.getElementById('topbar').innerHTML = `
      <div class="patient">
        <h1>${esc(meta ? meta.label : 'Patient')}</h1>
        <div class="facts">${facts}</div>
        ${meta && meta.description ? `<p class="fact" style="margin:10px 0 0;max-width:680px;line-height:1.5">${esc(meta.description)}</p>` : ''}
      </div>
      <div class="headline">
        <span class="status-pill s-${result.status}">${esc(OVERALL_LABEL[result.status])}</span>
        <div class="sub">Fully vaccinated (strict): <b>${result.fully_vaccinated ? 'Yes' : 'No'}</b></div>
      </div>`;
  }

  function renderKpis(record, result) {
    const eligible = result.by_series.filter((s) => s.eligible);
    const upToDate = eligible.filter((s) => s.up_to_date_for_age).length;
    const behind = eligible.length - upToDate;
    const complete = result.by_series.filter((s) => s.status === 'complete').length;
    const outOfSchedule = result.by_series.reduce((a, s) => a + s.doses_recorded.filter((d) => !d.within_schedule).length, 0);
    const unmatched = result.unmatched_doses.length;
    const duplicates = result.duplicate_doses.length;
    const covered = result.by_antigen.filter((a) => a.covered).length;

    const kpi = (label, value, note, opts = {}) => `
      <div class="kpi ${opts.accent ? 'accent' : ''} ${opts.warn ? 'warn' : ''}">
        <div class="k-label">${esc(label)}</div>
        <div class="k-value">${value}</div>
        <div class="k-note">${note}</div>
      </div>`;

    document.getElementById('kpis').innerHTML = [
      kpi('Up to date for age', `${upToDate}<small>/${eligible.length} series</small>`,
        behind ? `${behind} series behind` : 'no gaps that are due', { accent: true, warn: behind > 0 }),
      kpi('Series complete', `${complete}<small>/${eligible.length}</small>`, 'all doses given at all ages'),
      kpi('Doses recorded', `${record.immunisations.length}`,
        `${outOfSchedule} out-of-schedule &middot; ${unmatched} unmatched &middot; ${duplicates} duplicate`,
        { warn: outOfSchedule + unmatched + duplicates > 0 }),
      kpi('Antigens covered', `${covered}<small>/${result.by_antigen.length}</small>`, 'diseases protected against', { accent: true }),
    ].join('');
  }

  function renderPanels(record, result) {
    document.getElementById('panels').innerHTML =
      panelRecordedDoses(record, result) +
      panelConformance(result) +
      panelCoverage(result) +
      panelNotes(result);
  }

  // Panel 1 - the decomposition layer: each recorded dose -> product -> class + antigens.
  function panelRecordedDoses(record, result) {
    // Map each immunisation (date order) to its engine outcome.
    const queues = new Map(); // product_class -> recorded doses (in order) for that series
    for (const s of result.by_series) {
      const series = seriesById.get(s.series_id);
      const q = queues.get(series.product_class) || [];
      queues.set(series.product_class, q.concat(s.doses_recorded));
    }
    const imms = record.immunisations.slice().sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : 0));

    // Duplicate echoes are removed from series matching; track them so each
    // recorded dose row gets the right outcome.
    const pendingDups = result.duplicate_doses.slice();
    const isDup = (imm) => {
      const i = pendingDups.findIndex((d) =>
        d.date === imm.date && d.vaccine_code === imm.vaccine_code && (d.procedure_code || null) === (imm.procedure_code || null));
      if (i >= 0) return pendingDups.splice(i, 1)[0];
      return null;
    };

    const rows = imms.map((imm) => {
      const product = productIndex.get(imm.vaccine_code) || null;
      const cls = product ? product.product_class : null;
      const age = G.ageBetween(G.parseDate(record.dob), G.parseDate(imm.date));
      let badge, reason = '', rowClass = '', flags = [];
      const dup = isDup(imm);
      if (dup) { badge = ['muted', 'duplicate']; reason = `echo of the dose on ${dup.duplicate_of} (same procedure code)`; rowClass = 'row-dup'; }
      else if (!cls) { badge = ['red', 'unmatched']; reason = 'unknown product code'; rowClass = 'row-unmatched'; }
      else if (!scheduleClasses.has(cls)) { badge = ['red', 'unmatched']; reason = `class "${cls}" has no series here`; rowClass = 'row-unmatched'; }
      else {
        const rec = (queues.get(cls) || []).shift();
        if (rec && !rec.within_schedule) { badge = ['warn', 'out of schedule']; reason = rec.schedule_notes[0] || ''; rowClass = 'row-out'; }
        else badge = ['ok', 'counted'];
        if (rec) flags = rec.flags || [];
      }
      const antigens = product ? product.antigens.map((a) => antigenChip(a, true)).join('') : '<span class="code">-</span>';
      const flagHtml = flags.map((f) => `<div class="dose-flag">? ${esc(f)}</div>`).join('');
      return `
        <tr class="${rowClass}">
          <td class="mono num">${esc(imm.date)}</td>
          <td class="num">${esc(age)}</td>
          <td>${esc(imm.display || imm.vaccine_code)}<div class="code">${esc(imm.vaccine_code)}</div></td>
          <td>${cls ? `<span class="chip cls">${esc(cls)}</span>` : '<span class="code">-</span>'}</td>
          <td>${antigens}</td>
          <td><span class="badge ${badge[0]}">${esc(badge[1])}</span>${reason ? `<div class="dose-reason ${badge[0] === 'red' ? 'red' : ''}">${esc(reason)}</div>` : ''}${flagHtml}</td>
        </tr>`;
    }).join('');

    return panel(1, 'Recorded doses', 'product &rarr; class &amp; antigens', `
      <table class="tbl">
        <thead><tr><th>Date</th><th>Age</th><th>Product</th><th>Class</th><th>Antigens</th><th>Outcome</th></tr></thead>
        <tbody>${rows || emptyRow(6, 'No doses recorded.')}</tbody>
      </table>`, 'wide');
  }

  // Panel 2 - conformance: per series, matched by product class.
  function panelConformance(result) {
    const rows = result.by_series.map((s) => {
      const series = seriesById.get(s.series_id);
      const [bcls, blabel] = SERIES_BADGE[s.status];
      const doses = s.eligible ? `<span class="num">${s.doses_valid}/${s.doses_due}</span> <span class="code">/ ${s.doses_expected}</span>` : '<span class="code">-</span>';
      let forAge;
      if (!s.eligible) forAge = '<span class="badge ghost">N/A</span>';
      else if (s.up_to_date_for_age) forAge = '<span class="badge ok">up to date</span>';
      else forAge = '<span class="badge warn">behind</span>';
      return `
        <tr>
          <td>${esc(s.display_name)}${s.eligibility_uncertain ? '<div class="uncertain">eligibility uncertain</div>' : ''}</td>
          <td><span class="chip cls">${esc(series.product_class)}</span></td>
          <td>${doses}</td>
          <td><span class="badge ${bcls}">${esc(blabel)}</span></td>
          <td>${forAge}</td>
        </tr>`;
    }).join('');

    return panel(2, 'Conformance by series', 'matched by product class', `
      <table class="tbl">
        <thead><tr><th>Series</th><th>Class</th><th>Valid / due / total</th><th>Status</th><th>For age</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>`);
  }

  // Panel 3 - coverage: which diseases is the child protected against?
  function panelCoverage(result) {
    const items = result.by_antigen.map((a) => `
      <div class="cov-item ${a.covered ? 'on' : 'off'}" title="${a.by_doses.map((d) => esc(d.display || d.vaccine_code)).join(', ') || 'not covered'}">
        <span class="cov-dot"></span><span class="cov-name">${esc((antigenById.get(a.id) || {}).display_name || a.id)}</span>
      </div>`).join('');
    const covered = result.by_antigen.filter((a) => a.covered).length;
    return panel(3, 'Antigen coverage', `${covered} of ${result.by_antigen.length} diseases`, `<div class="cov-grid">${items}</div>`);
  }

  // Anomalies: out-of-schedule, unmatched, eligibility uncertainty.
  function panelNotes(result) {
    const items = [];
    for (const s of result.by_series) {
      for (const d of s.doses_recorded) {
        if (!d.within_schedule) items.push(`<li><span class="when">${esc(d.date)}</span><span><b>${esc(s.display_name)}</b> &middot; ${esc(d.schedule_notes[0] || 'outside standard schedule')}</span></li>`);
        for (const f of d.flags || []) items.push(`<li><span class="when">${esc(d.date)}</span><span><b>${esc(s.display_name)}</b> &middot; ${esc(f)}</span></li>`);
      }
      if (s.eligibility_uncertain) items.push(`<li><span class="when">-</span><span><b>${esc(s.display_name)}</b> &middot; ${esc(s.notes[0] || 'eligibility uncertain')}</span></li>`);
    }
    for (const u of result.unmatched_doses) {
      items.push(`<li><span class="when">${esc(u.date)}</span><span><b>${esc(u.display || u.vaccine_code)}</b> <span class="code">${esc(u.vaccine_code)}</span> &middot; ${esc(u.reason)}</span></li>`);
    }
    for (const dup of result.duplicate_doses) {
      items.push(`<li><span class="when">${esc(dup.date)}</span><span><b>${esc(dup.display || dup.vaccine_code)}</b> &middot; likely duplicate (echo) of the dose on ${esc(dup.duplicate_of)} - same procedure code</span></li>`);
    }
    const body = items.length
      ? `<ul class="notes">${items.join('')}</ul>`
      : `<div class="empty">No anomalies - every recorded dose matched a series and fell within the standard schedule.</div>`;
    return panel('!', 'Anomalies', 'out-of-schedule, unmatched, duplicates &amp; flags', body, 'wide');
  }

  // --- helpers --------------------------------------------------------------

  function panel(step, title, sub, body, extra = '') {
    return `
      <section class="panel ${extra}">
        <div class="panel-head">
          <span class="panel-step">${step}</span>
          <span class="panel-title">${esc(title)} <span class="muted">&middot; ${sub}</span></span>
        </div>
        <div class="panel-body">${body}</div>
      </section>`;
  }
  const emptyRow = (cols, msg) => `<tr><td colspan="${cols}"><span class="empty">${esc(msg)}</span></td></tr>`;

  // --- Custom patient mode --------------------------------------------------

  // The date a ticked schedule dose is recorded at: its target-age date once
  // that has been reached, otherwise the evaluation date (i.e. given as soon as
  // it became due). Returns null if the dose is not yet due - using earliest_age
  // so this matches the engine's notion of "due". Keeping these aligned means a
  // dose is tickable exactly when the engine would otherwise flag it as a gap.
  function customDoseDate(dose, dobD, evalD) {
    const earliest = G.ageOffsetToDate(dose.earliest_age || dose.target_age, dobD);
    if (earliest.getTime() > evalD.getTime()) return null;
    const target = G.ageOffsetToDate(dose.target_age, dobD);
    return target.getTime() <= evalD.getTime() ? target : evalD;
  }

  // Turn the builder controls into a record the engine understands. Ticked
  // schedule doses become an on-time dose of the series' representative product;
  // a tick only counts if that dose is actually due by the evaluation date.
  function buildCustomRecord() {
    const dobD = G.parseDate(customState.dob);
    const evalD = G.parseDate(customState.evaluatedAt);
    const imms = [];
    for (const series of schedule.series) {
      const rep = repProductByClass.get(series.product_class);
      if (!rep) continue;
      for (const dose of series.dose) {
        const date = customDoseDate(dose, dobD, evalD);
        if (date && customState.scheduleDoses.has(series.id + '#' + dose.number)) {
          imms.push({ date: G.fmtDate(date), vaccine_code: rep.code, display: rep.display, dose_number: dose.number });
        }
      }
    }
    for (const e of customState.extraDoses) imms.push({ date: e.date, vaccine_code: e.code, display: e.display, dose_number: null });
    imms.sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : 0));
    const gender = customState.gender === 'unknown' ? null : customState.gender;
    return { patientId: 'custom', dob: customState.dob, gender, immunisations: imms };
  }

  function renderCustom() {
    const record = buildCustomRecord();
    const evaluatedAt = customState.evaluatedAt;
    const result = G.evaluate(record, schedule, products, evaluatedAt);
    renderTopbar(record, evaluatedAt, result, { label: 'Custom patient' });
    renderBuilder(record);
    renderKpis(record, result);
    renderPanels(record, result);
  }

  function renderBuilder() {
    const dobD = G.parseDate(customState.dob);
    const evalD = G.parseDate(customState.evaluatedAt);

    const sexSel = ['female', 'male', 'other', 'unknown']
      .map((g) => `<option value="${g}" ${customState.gender === g ? 'selected' : ''}>${g[0].toUpperCase() + g.slice(1)}</option>`).join('');
    const presets = AGE_PRESETS.map(([code, label]) => `<button class="age-btn" data-age="${code}">${esc(label)}</button>`).join('');

    // Schedule checklist - one row per series, a chip per dose. Not-yet-due doses
    // are disabled, so increasing the age unlocks more of the schedule.
    const doseRows = schedule.series.map((series) => {
      const chips = series.dose.map((dose) => {
        const key = series.id + '#' + dose.number;
        const due = customDoseDate(dose, dobD, evalD) !== null;
        const on = due && customState.scheduleDoses.has(key);
        const title = due ? `Dose ${dose.number} - target ${esc(dose.target_age)}` : `Dose ${dose.number} - not due until ${esc(dose.target_age)}`;
        return `<button class="dosechip ${on ? 'on' : ''}" data-dose="${key}" ${due ? '' : 'disabled'} title="${title}">${esc(abbreviateAge(dose.target_age))}</button>`;
      }).join('');
      return `<div class="dose-series"><span class="ds-name">${esc(series.display_name)} <span class="chip cls">${esc(series.product_class)}</span></span><span class="ds-doses">${chips}</span></div>`;
    }).join('');

    const productOpts = products.product
      .map((p) => `<option value="${esc(p.code)}">${esc(shortName(p.display))} (${esc(p.product_class)})</option>`).join('')
      + `<option value="__unknown__">Unknown product (made-up code)</option>`;
    const extraList = customState.extraDoses.length
      ? customState.extraDoses.map((e, i) => `<div class="extra-item"><button class="rm" data-rm="${i}" title="remove">&times;</button><span class="mono">${esc(e.date)}</span><span>${esc(shortName(e.display))}</span></div>`).join('')
      : '<div class="empty">None. Add one here to try a too-early / too-late dose, or an off-schedule product (e.g. Pediacel 5-in-1).</div>';

    const body = `
      <div class="builder-row">
        <label class="field"><span>Date of birth</span><input type="date" id="dob" value="${esc(customState.dob)}"></label>
        <label class="field"><span>Evaluate at</span><input type="date" id="evalAt" value="${esc(customState.evaluatedAt)}"></label>
        <label class="field"><span>Sex</span><select id="sex">${sexSel}</select></label>
        <div class="field"><span>Quick age</span><div class="age-presets">${presets}</div></div>
      </div>
      <div class="builder-sub">Doses given <span class="hint">tap the appointments this child attended - only doses due by the evaluation date are selectable</span></div>
      <div class="dose-grid">${doseRows}</div>
      <div class="builder-sub">Off-schedule or unknown doses <span class="hint">for edge cases - any product on any date</span></div>
      <div class="extra-add">
        <select id="extraProduct">${productOpts}</select>
        <input type="date" id="extraDate" value="${esc(customState.evaluatedAt)}">
        <button class="btn" id="extraAdd">Add dose</button>
      </div>
      <div class="extra-list">${extraList}</div>`;

    document.getElementById('builder').innerHTML = `
      <section class="panel builder-panel">
        <div class="panel-head"><span class="panel-step">&#9998;</span><span class="panel-title">Build a patient <span class="muted">&middot; set age &amp; doses; everything below updates live</span></span></div>
        <div class="panel-body">${body}</div>
      </section>`;
    attachBuilder();
  }

  function attachBuilder() {
    const b = document.getElementById('builder');
    b.querySelector('#dob').addEventListener('change', (e) => { customState.dob = e.target.value; renderCustom(); });
    b.querySelector('#evalAt').addEventListener('change', (e) => { customState.evaluatedAt = e.target.value; renderCustom(); });
    b.querySelector('#sex').addEventListener('change', (e) => { customState.gender = e.target.value; renderCustom(); });
    b.querySelectorAll('[data-age]').forEach((btn) => btn.addEventListener('click', () => setAge(btn.dataset.age)));
    b.querySelectorAll('[data-dose]').forEach((btn) => btn.addEventListener('click', () => {
      const k = btn.dataset.dose;
      if (customState.scheduleDoses.has(k)) customState.scheduleDoses.delete(k); else customState.scheduleDoses.add(k);
      renderCustom();
    }));
    b.querySelector('#extraAdd').addEventListener('click', addExtra);
    b.querySelectorAll('[data-rm]').forEach((btn) => btn.addEventListener('click', () => { customState.extraDoses.splice(Number(btn.dataset.rm), 1); renderCustom(); }));
  }

  // Quick-age buttons set DOB relative to the evaluation date.
  function setAge(code) {
    const evalD = G.parseDate(customState.evaluatedAt);
    let dob;
    if (code === 'new') dob = evalD;
    else if (code[0] === 'w') dob = G.addDays(evalD, -7 * parseInt(code.slice(1), 10));
    else dob = G.addMonths(evalD, -parseInt(code.slice(1), 10));
    customState.dob = G.fmtDate(dob);
    renderCustom();
  }

  function addExtra() {
    const sel = document.getElementById('extraProduct').value;
    const date = document.getElementById('extraDate').value;
    if (!date) return;
    let code, display;
    if (sel === '__unknown__') { code = '00000000000000000'; display = 'Unknown product'; }
    else { const p = productIndex.get(sel); code = p.code; display = p.display; }
    customState.extraDoses.push({ code, display, date });
    renderCustom();
  }

  buildSidebar();
  render();
})();
