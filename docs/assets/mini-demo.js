/* A pared-back, read-only version of the interactive demo, embedded on the
   home page. Reuses the demo's vendored engine and generated data: pick an
   example patient, see the headline verdict, a compact per-series breakdown,
   and antigen coverage. The full demo (build-your-own, timeline, detail) lives
   at /demo/. Loaded on every page via extra_javascript but no-ops unless the
   #gb-mini-demo container is present (i.e. only on the home page). */
(function () {
  "use strict";

  var root = document.getElementById("gb-mini-demo");
  if (!root) return;

  // Resolve the demo assets relative to this page (works under the GitHub
  // Pages sub-path, where an absolute /demo/ would be wrong).
  var base = root.getAttribute("data-demo-base") || "demo/";

  var LABEL = {
    up_to_date_for_age: "Up to date for age",
    behind_for_age: "Not up to date for age",
    unvaccinated: "Not up to date for age",
    unknown: "Status unknown",
  };
  // Headline pill is binary green/red (up to date vs not); the per-series tags
  // keep the amber "behind" nuance.
  var TONE = {
    up_to_date_for_age: "ok",
    behind_for_age: "bad",
    unvaccinated: "bad",
    unknown: "muted",
  };
  // On-schedule first, then the "messy record" scenarios.
  var ORDER = [
    "six-month-fully-vaccinated",
    "behind-for-age-toddler",
    "out-of-schedule-doses",
    "unmatched-doses",
    "mmr-both-doses",
    "duplicate-echo",
    "dose-number-mismatch",
  ];

  function esc(s) {
    return String(s == null ? "" : s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  function loadScript(src) {
    return new Promise(function (resolve, reject) {
      var s = document.createElement("script");
      s.src = src;
      s.onload = resolve;
      s.onerror = function () { reject(new Error("failed to load " + src)); };
      document.head.appendChild(s);
    });
  }

  loadScript(base + "engine.js")
    .then(function () { return loadScript(base + "data.js"); })
    .then(init)
    .catch(function () {
      root.innerHTML =
        '<p class="gb-mini__err">The embedded demo could not be loaded. ' +
        '<a href="' + esc(base) + 'index.html">Open the full demo</a> instead.</p>';
    });

  function init() {
    var G = window.GREENBOOK;
    var GB = window.Greenbook;
    if (!G || !GB) return;

    var fixtures = [];
    ORDER.forEach(function (id) {
      var f = G.fixtures.find(function (x) { return x.id === id; });
      if (f) fixtures.push(f);
    });
    G.fixtures.forEach(function (f) {
      if (ORDER.indexOf(f.id) < 0) fixtures.push(f);
    });

    var bar = root.querySelector(".gb-mini__bar");
    fixtures.forEach(function (fx) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "gb-mini__tab";
      b.textContent = fx.label || fx.id;
      b.addEventListener("click", function () { select(fx, b); });
      bar.appendChild(b);
    });

    if (fixtures.length) select(fixtures[0], bar.firstChild);
  }

  function select(fx, btn) {
    var G = window.GREENBOOK;
    var GB = window.Greenbook;
    root.querySelectorAll(".gb-mini__tab").forEach(function (b) {
      b.classList.toggle("is-active", b === btn);
    });
    var result = GB.evaluate(fx.record, G.schedule, G.products, fx.evaluatedAt);
    renderHead(fx, result);
    renderSeries(result);
  }

  function renderHead(fx, r) {
    var GB = window.Greenbook;
    var age = GB.ageBetween(GB.parseDate(fx.record.dob), GB.parseDate(fx.evaluatedAt));
    var antigens = (r.by_antigen || []);
    var covered = antigens.filter(function (a) { return a.covered; }).length;
    var recorded = (fx.record.immunisations || []).length;

    var stats =
      '<span class="gb-stat"><b>' + recorded + "</b> dose" + (recorded === 1 ? "" : "s") + " recorded</span>" +
      '<span class="gb-stat"><b>' + covered + "</b>/" + antigens.length + " antigens covered</span>";
    if (r.unmatched_doses && r.unmatched_doses.length)
      stats += '<span class="gb-stat gb-stat--warn"><b>' + r.unmatched_doses.length + "</b> unmatched</span>";
    if (r.duplicate_doses && r.duplicate_doses.length)
      stats += '<span class="gb-stat gb-stat--warn"><b>' + r.duplicate_doses.length + "</b> duplicate</span>";

    root.querySelector(".gb-mini__head").innerHTML =
      '<div class="gb-mini__desc">' + esc(fx.description) + "</div>" +
      '<div class="gb-mini__status">' +
        '<span class="gb-pill gb-pill--' + (TONE[r.status] || "muted") + '">' + esc(LABEL[r.status] || r.status) + "</span>" +
        '<span class="gb-mini__strict">Fully vaccinated (strict): <strong>' + (r.fully_vaccinated ? "yes" : "no") + "</strong></span>" +
      "</div>" +
      '<div class="gb-mini__meta">' +
        "<span>DOB <strong>" + esc(fx.record.dob) + "</strong></span>" +
        "<span>Age <strong>" + esc(age) + "</strong></span>" +
        "<span>Sex <strong>" + esc(fx.record.gender || "—") + "</strong></span>" +
        "<span>Evaluated <strong>" + esc(fx.evaluatedAt) + "</strong></span>" +
      "</div>" +
      '<div class="gb-mini__stats">' + stats + "</div>";
  }

  function renderSeries(r) {
    var box = root.querySelector(".gb-mini__series");
    var rows = r.by_series
      .filter(function (s) { return s.doses_due > 0 || s.doses_valid > 0; })
      .map(function (s) {
        var tag = s.up_to_date_for_age
          ? '<span class="gb-tag gb-tag--ok">up to date</span>'
          : '<span class="gb-tag gb-tag--warn">behind</span>';
        return (
          '<div class="gb-row">' +
            '<span class="gb-row__name">' + esc(s.display_name) + "</span>" +
            '<span class="gb-row__count">' + s.doses_valid + "/" + s.doses_due +
              " <small>of " + s.doses_expected + "</small></span>" +
            tag +
          "</div>"
        );
      })
      .join("");
    box.innerHTML = rows || '<div class="gb-row gb-row--empty">No doses due yet at this age.</div>';
  }
})();
