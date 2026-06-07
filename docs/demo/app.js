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

  // --- State & selection ----------------------------------------------------

  let currentId = fixtures[0].id;

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
    };
    return HINTS[f.id] || '';
  }

  // --- Render ---------------------------------------------------------------

  function render() {
    document.querySelectorAll('.preset[data-id]').forEach((b) =>
      b.classList.toggle('active', b.dataset.id === currentId)
    );
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
        `${outOfSchedule} outside schedule &middot; ${unmatched} unmatched`, { warn: outOfSchedule + unmatched > 0 }),
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

    const rows = imms.map((imm) => {
      const product = productIndex.get(imm.vaccine_code) || null;
      const cls = product ? product.product_class : null;
      const age = G.ageBetween(G.parseDate(record.dob), G.parseDate(imm.date));
      let badge, reason = '', rowClass = '';
      if (!cls) { badge = ['red', 'unmatched']; reason = 'unknown product code'; rowClass = 'row-unmatched'; }
      else if (!scheduleClasses.has(cls)) { badge = ['red', 'unmatched']; reason = `class "${cls}" has no series here`; rowClass = 'row-unmatched'; }
      else {
        const rec = (queues.get(cls) || []).shift();
        if (rec && !rec.within_schedule) { badge = ['warn', 'out of schedule']; reason = rec.schedule_notes[0] || ''; rowClass = 'row-out'; }
        else badge = ['ok', 'counted'];
      }
      const antigens = product ? product.antigens.map((a) => antigenChip(a, true)).join('') : '<span class="code">-</span>';
      return `
        <tr class="${rowClass}">
          <td class="mono num">${esc(imm.date)}</td>
          <td class="num">${esc(age)}</td>
          <td>${esc(imm.display || imm.vaccine_code)}<div class="code">${esc(imm.vaccine_code)}</div></td>
          <td>${cls ? `<span class="chip cls">${esc(cls)}</span>` : '<span class="code">-</span>'}</td>
          <td>${antigens}</td>
          <td><span class="badge ${badge[0]}">${esc(badge[1])}</span>${reason ? `<div class="dose-reason ${badge[0] === 'red' ? 'red' : ''}">${esc(reason)}</div>` : ''}</td>
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
      }
      if (s.eligibility_uncertain) items.push(`<li><span class="when">-</span><span><b>${esc(s.display_name)}</b> &middot; ${esc(s.notes[0] || 'eligibility uncertain')}</span></li>`);
    }
    for (const u of result.unmatched_doses) {
      items.push(`<li><span class="when">${esc(u.date)}</span><span><b>${esc(u.display || u.vaccine_code)}</b> <span class="code">${esc(u.vaccine_code)}</span> &middot; ${esc(u.reason)}</span></li>`);
    }
    const body = items.length
      ? `<ul class="notes">${items.join('')}</ul>`
      : `<div class="empty">No anomalies - every recorded dose matched a series and fell within the standard schedule.</div>`;
    return panel('!', 'Anomalies', 'out-of-schedule &amp; unmatched', body, 'wide');
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

  buildSidebar();
  render();
})();
