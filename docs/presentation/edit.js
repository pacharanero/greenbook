#!/usr/bin/env node
/**
 * Reveal-aware in-browser text editor for this deck.
 *
 *   node docs/presentation/edit.js [presentation.html]
 *
 * Why a project-local editor instead of the generic revealjs-skill one?
 * Two reveal.js-specific problems the generic editor hits:
 *
 *  1. Clean save. Reveal.js heavily mutates the DOM at runtime (adds
 *     .backgrounds/.controls/.progress, present/past/future classes, aria-*
 *     attributes, inline transform styles, ...). Serialising the *live* DOM
 *     therefore writes all that cruft back to the file. Here we instead keep a
 *     pristine copy of the file as served, copy only the edited text into it,
 *     and save that - so the diff is just your wording changes.
 *
 *  2. Editing under a scaled transform. Reveal scales `.slides` with
 *     `transform: scale()` to fit the window; `contenteditable` under a scaled
 *     transform misbehaves in some browsers (caret offset, clicks not landing).
 *     While editing we pin reveal to scale 1 (Reveal.configure minScale/maxScale)
 *     so the caret behaves.
 */

const http = require('http');
const fs = require('fs');
const path = require('path');
const { exec } = require('child_process');

const PORT = process.env.PORT ? Number(process.env.PORT) : 3456;
const htmlFile = process.argv[2] || path.join(__dirname, 'presentation.html');
const htmlFilePath = path.resolve(htmlFile);

if (!fs.existsSync(htmlFilePath)) {
  console.error(`File not found: ${htmlFilePath}`);
  process.exit(1);
}

// Injected editor: toolbar + the edit/save logic. Kept dependency-free.
const editorScript = `
<style>
  .ed-toolbar {
    position: fixed; top: 10px; right: 10px; z-index: 2147483647;
    background: #14201B; padding: 8px 12px; border-radius: 8px;
    box-shadow: 0 2px 12px rgba(0,0,0,.35); display: flex; gap: 8px; align-items: center;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  }
  .ed-toolbar button { border: none; border-radius: 5px; padding: 8px 14px; cursor: pointer; font-size: 14px; font-weight: 600; color: #fff; }
  .ed-toolbar button.save { background: #1E6F52; }
  .ed-toolbar button.save:hover { background: #16573F; }
  .ed-toolbar button.secondary { background: #46555C; }
  .ed-toolbar .status { color: #BFD8C9; font-size: 13px; margin-left: 6px; }
  .ed-toast { position: fixed; bottom: 28px; left: 50%; transform: translateX(-50%); background: #14201B; color: #fff;
    padding: 11px 22px; border-radius: 8px; font: 14px -apple-system, sans-serif; z-index: 2147483647; opacity: 0;
    transition: opacity .25s; pointer-events: none; }
  .ed-toast.err { background: #C2602F; }
  .ed-toast.show { opacity: 1; }
  [contenteditable="true"] { outline: 2px dashed transparent; transition: outline-color .15s; }
  [contenteditable="true"]:hover { outline-color: rgba(30,111,82,.5); }
  [contenteditable="true"]:focus { outline-color: #1E6F52; outline-style: solid; }
</style>
<div class="ed-toast" id="edToast"></div>
<div class="ed-toolbar">
  <button class="save" onclick="edSave()">Save</button>
  <button class="secondary" onclick="location.reload()">Reload</button>
  <span class="status" id="edStatus">Click any text to edit</span>
</div>
<script>
(function () {
  // Only block-level text elements inside the slides are editable. This avoids
  // reveal's injected chrome and avoids nested-editable confusion with layout divs.
  var SELECTOR = '.reveal .slides h1, .reveal .slides h2, .reveal .slides h3, .reveal .slides h4, .reveal .slides p, .reveal .slides li, .reveal .slides blockquote';

  function toast(msg, isErr) {
    var t = document.getElementById('edToast');
    t.textContent = msg; t.className = 'ed-toast show' + (isErr ? ' err' : '');
    setTimeout(function () { t.className = 'ed-toast'; }, 2200);
  }

  var params = new URLSearchParams(location.search);
  if (params.has('saved')) { toast('Saved'); history.replaceState(null, '', location.pathname); }
  if (params.has('error')) { toast('Save error: ' + params.get('error'), true); history.replaceState(null, '', location.pathname); }

  function ready(fn) {
    if (typeof Reveal !== 'undefined' && Reveal.isReady && Reveal.isReady()) return fn();
    setTimeout(function () { ready(fn); }, 60);
  }

  ready(function () {
    // Pin scale to 1 so contenteditable caret/clicks behave under reveal's transform.
    try { Reveal.configure({ minScale: 1, maxScale: 1, transition: 'none' }); } catch (e) {}

    var live = Array.prototype.slice.call(document.querySelectorAll(SELECTOR));
    live.forEach(function (el, i) {
      el.setAttribute('contenteditable', 'true');
      el.dataset.edId = String(i);   // stable index, matched against the pristine file on save
    });

    var changed = false;
    document.addEventListener('input', function (e) {
      if (e.target.getAttribute && e.target.getAttribute('contenteditable') === 'true') {
        changed = true; document.getElementById('edStatus').textContent = 'Unsaved changes';
      }
    });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && document.activeElement && document.activeElement.getAttribute('contenteditable') === 'true') {
        document.activeElement.blur();
      }
    });
    window.addEventListener('beforeunload', function (e) { if (changed) { e.preventDefault(); e.returnValue = ''; } });

    // Clean save: fetch the pristine file, copy edited innerHTML into the matching
    // elements by edId, and save *that* - never the reveal-mutated live DOM.
    window.edSave = function () {
      fetch('/__raw').then(function (r) { return r.text(); }).then(function (raw) {
        var doc = new DOMParser().parseFromString(raw, 'text/html');
        var pristine = doc.querySelectorAll(SELECTOR);
        if (pristine.length !== live.length) {
          throw new Error('element count mismatch (' + pristine.length + ' vs ' + live.length + ') - reload and retry');
        }
        live.forEach(function (el, i) {
          var clone = el.cloneNode(true);
          clone.removeAttribute('contenteditable');
          clone.removeAttribute('data-ed-id');
          pristine[i].innerHTML = clone.innerHTML;
        });
        var out = '<!DOCTYPE html>\\n' + doc.documentElement.outerHTML + '\\n';
        return fetch('/__save', { method: 'POST', headers: { 'Content-Type': 'text/html' }, body: out });
      }).then(function (resp) {
        if (!resp.ok) throw new Error('server returned ' + resp.status);
        changed = false; location.href = location.pathname + '?saved=1' + location.hash;
      }).catch(function (err) {
        location.href = location.pathname + '?error=' + encodeURIComponent(err.message) + location.hash;
      });
    };
  });
})();
</script>
`;

const MIME = {
  '.css': 'text/css', '.js': 'application/javascript', '.json': 'application/json',
  '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.gif': 'image/gif',
  '.svg': 'image/svg+xml', '.ico': 'image/x-icon', '.woff': 'font/woff', '.woff2': 'font/woff2',
};

const server = http.createServer((req, res) => {
  const reqPath = req.url.split('?')[0];

  // Pristine, unmodified file (used by the client to build a clean save).
  if (req.method === 'GET' && reqPath === '/__raw') {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(fs.readFileSync(htmlFilePath, 'utf8'));
    return;
  }

  if (req.method === 'POST' && reqPath === '/__save') {
    let body = '';
    req.on('data', c => (body += c));
    req.on('end', () => {
      try {
        fs.writeFileSync(htmlFilePath, body, 'utf8');
        console.log(`✓ Saved ${htmlFilePath}`);
        res.writeHead(200); res.end('OK');
      } catch (err) {
        console.error('Save failed:', err);
        res.writeHead(500); res.end('error');
      }
    });
    return;
  }

  // Serve the deck with the editor injected.
  if (req.method === 'GET' && (reqPath === '/' || reqPath === '/' + path.basename(htmlFilePath))) {
    let html = fs.readFileSync(htmlFilePath, 'utf8');
    html = html.includes('</body>') ? html.replace('</body>', editorScript + '</body>') : html + editorScript;
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(html);
    return;
  }

  // Static assets (styles.css, etc.) from the deck's directory.
  const baseDir = path.dirname(htmlFilePath);
  const filePath = path.join(baseDir, reqPath);
  if (filePath.startsWith(baseDir) && fs.existsSync(filePath) && fs.statSync(filePath).isFile()) {
    res.writeHead(200, { 'Content-Type': MIME[path.extname(filePath).toLowerCase()] || 'application/octet-stream' });
    res.end(fs.readFileSync(filePath));
    return;
  }

  res.writeHead(404); res.end('Not found');
});

server.listen(PORT, () => {
  const url = `http://localhost:${PORT}`;
  console.log(`\n  Reveal editor: ${url}`);
  console.log(`  Editing:       ${htmlFilePath}\n`);
  console.log('  Click any text to edit, Esc to deselect, then Save. Ctrl+C to stop.\n');
  const open = process.platform === 'darwin' ? 'open' : process.platform === 'win32' ? 'start' : 'xdg-open';
  exec(`${open} ${url}`);
});
