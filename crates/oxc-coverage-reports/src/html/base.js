/* oxc-coverage-instrument HTML reporter client script.
 *
 * Self-contained, no external dependencies, no network access. Provides:
 *   1. theme toggle (auto / light / dark, persisted to localStorage),
 *   2. sortable index tables (click or keyboard, aria-sort updates).
 *
 * Syntax highlighting of source view is done server-side in Rust via
 * syntect; the detail-page HTML arrives pre-spanned so there is no
 * client-side tokenizer or paint-time work here.
 *
 * The page is fully usable without JS: missing JS just means no sort
 * and no explicit theme toggle (prefers-color-scheme still works).
 * Every feature is progressive enhancement.
 *
 * Note: this script uses DOM methods (createElement / textContent /
 * appendChild) exclusively. It never assigns HTML strings, so it cannot
 * introduce XSS even if source files contain hostile characters.
 */
(function () {
  'use strict';

  // ---------- Theme toggle ----------------------------------------------
  var STORAGE_KEY = 'oxc-coverage-theme';

  function readStoredTheme() {
    try { return window.localStorage.getItem(STORAGE_KEY); }
    catch (_e) { return null; }
  }

  function writeStoredTheme(value) {
    try {
      if (value === null) { window.localStorage.removeItem(STORAGE_KEY); }
      else { window.localStorage.setItem(STORAGE_KEY, value); }
    } catch (_e) { /* private mode, ignore */ }
  }

  function applyTheme(value) {
    var root = document.documentElement;
    if (value === 'light' || value === 'dark') {
      root.setAttribute('data-theme', value);
    } else {
      root.removeAttribute('data-theme');
    }
  }

  // Apply stored theme immediately to avoid a flash of the wrong palette.
  var storedTheme = readStoredTheme();
  applyTheme(storedTheme);

  function buildThemeToggle() {
    var header = document.querySelector('header.summary');
    if (!header) { return; }
    var wrap = document.createElement('div');
    wrap.className = 'theme-toggle';
    wrap.setAttribute('role', 'group');
    wrap.setAttribute('aria-label', 'Color theme');
    var current = storedTheme || 'auto';
    var options = [
      ['auto', 'Auto'],
      ['light', 'Light'],
      ['dark', 'Dark']
    ];
    options.forEach(function (opt) {
      var value = opt[0];
      var label = opt[1];
      var btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'theme-toggle__btn';
      btn.textContent = label;
      btn.setAttribute('data-theme-value', value);
      btn.setAttribute('aria-pressed', String(value === current));
      btn.addEventListener('click', function () {
        if (value === 'auto') {
          writeStoredTheme(null);
          applyTheme(null);
        } else {
          writeStoredTheme(value);
          applyTheme(value);
        }
        var buttons = wrap.querySelectorAll('button[data-theme-value]');
        for (var i = 0; i < buttons.length; i++) {
          var b = buttons[i];
          b.setAttribute('aria-pressed',
            String(b.getAttribute('data-theme-value') === value));
        }
      });
      wrap.appendChild(btn);
    });
    header.appendChild(wrap);
  }

  // ---------- Sortable tables -------------------------------------------
  function initSortable() {
    var tables = document.querySelectorAll('table.coverage-summary');
    for (var t = 0; t < tables.length; t++) {
      enhanceTable(tables[t]);
    }
  }

  function enhanceTable(table) {
    var thead = table.tHead;
    var tbody = table.tBodies[0];
    if (!thead || !tbody) { return; }

    var rows = tbody.rows;
    for (var r = 0; r < rows.length; r++) {
      rows[r].setAttribute('data-original-index', String(r));
    }

    var ths = thead.querySelectorAll('th');
    for (var i = 0; i < ths.length; i++) {
      var th = ths[i];
      // <th> already exposes role="columnheader"; do not overwrite it.
      // tabindex + a keydown handler give us keyboard activation without
      // disturbing the table semantics for screen readers.
      th.classList.add('sortable');
      th.setAttribute('tabindex', '0');
      th.setAttribute('aria-sort', 'none');
      (function (idx, header) {
        var click = function () { applySort(table, idx, header); };
        header.addEventListener('click', click);
        header.addEventListener('keydown', function (ev) {
          if (ev.key === 'Enter' || ev.key === ' ' || ev.key === 'Spacebar') {
            ev.preventDefault();
            click();
          }
        });
      }(i, th));
    }
  }

  function applySort(table, colIdx, th) {
    var thead = table.tHead;
    var tbody = table.tBodies[0];
    if (!thead || !tbody) { return; }
    var rows = Array.prototype.slice.call(tbody.rows);
    var direction = th.getAttribute('aria-sort');
    var next;
    if (direction === 'ascending') { next = 'descending'; }
    else if (direction === 'descending') { next = 'none'; }
    else { next = 'ascending'; }

    var ths = thead.querySelectorAll('th');
    for (var i = 0; i < ths.length; i++) {
      ths[i].setAttribute('aria-sort', 'none');
    }

    if (next === 'none') {
      rows.sort(function (a, b) {
        return numericAttr(a, 'data-original-index') - numericAttr(b, 'data-original-index');
      });
    } else {
      th.setAttribute('aria-sort', next);
      var ascending = next === 'ascending';
      rows.sort(function (a, b) {
        var av = cellValue(a.cells[colIdx]);
        var bv = cellValue(b.cells[colIdx]);
        if (typeof av === 'number' && typeof bv === 'number') {
          return ascending ? av - bv : bv - av;
        }
        var as = String(av).toLowerCase();
        var bs = String(bv).toLowerCase();
        if (as < bs) { return ascending ? -1 : 1; }
        if (as > bs) { return ascending ? 1 : -1; }
        return 0;
      });
    }

    for (var k = 0; k < rows.length; k++) {
      tbody.appendChild(rows[k]);
    }
  }

  function numericAttr(el, name) {
    var v = parseInt(el.getAttribute(name) || '0', 10);
    return isNaN(v) ? 0 : v;
  }

  function cellValue(cell) {
    if (!cell) { return ''; }
    var text = cell.textContent || '';
    var pctMatch = text.match(/(-?\d+(?:\.\d+)?)\s*%/);
    if (pctMatch) { return parseFloat(pctMatch[1]); }
    var trimmed = text.trim();
    if (/^-?\d+(?:\.\d+)?$/.test(trimmed)) { return parseFloat(trimmed); }
    return trimmed;
  }

  // Source syntax highlighting is rendered server-side by syntect, so
  // there is no client-side tokenizer here. The detail-page <pre> cells
  // arrive with `<span class="stok-...">` markup already in the HTML.

  // ---------- File filter (index pages) ---------------------------------
  // Hooks: <input id="cov-filter"> + <div id="cov-filter-count"> +
  // <table id="cov-file-table">. Each tbody row carries data-file="..."
  // populated by the Rust render. Without JS the input is inert and the
  // count region stays empty; the table is fully usable.
  function initFilter() {
    var input = document.getElementById('cov-filter');
    var table = document.getElementById('cov-file-table');
    var count = document.getElementById('cov-filter-count');
    if (!input || !table) { return; }
    var tbody = table.tBodies[0];
    if (!tbody) { return; }
    var rows = Array.prototype.slice.call(tbody.rows);
    var total = rows.length;
    if (count && count.getAttribute('data-total')) {
      total = parseInt(count.getAttribute('data-total'), 10) || total;
    }

    var debounceHandle = 0;
    function applyFilter() {
      var q = input.value.trim().toLowerCase();
      var shown = 0;
      for (var i = 0; i < rows.length; i++) {
        var r = rows[i];
        var name = (r.getAttribute('data-file') || '').toLowerCase();
        var match = q === '' || name.indexOf(q) !== -1;
        r.style.display = match ? '' : 'none';
        if (match) { shown++; }
      }
      if (count) {
        if (q === '') {
          count.textContent = '';
        } else {
          count.textContent = shown + ' of ' + total + ' files match';
        }
      }
    }
    input.addEventListener('input', function () {
      if (debounceHandle) { clearTimeout(debounceHandle); }
      debounceHandle = setTimeout(applyFilter, 80);
    });
    input.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') {
        input.value = '';
        applyFilter();
      }
    });

    // Power-user shortcut: pressing `/` anywhere in the page (when not
    // already inside a form field) focuses the filter input. Guarded
    // against active form fields to avoid stealing focus from a user
    // typing into another input, and against contentEditable to avoid
    // intercepting normal text entry. Browse-mode screen-reader users
    // still see `/` as a "find in page" gesture because we only act
    // when the activeElement is the body itself.
    document.addEventListener('keydown', function (e) {
      if (e.key !== '/' || e.ctrlKey || e.metaKey || e.altKey) { return; }
      var ae = document.activeElement;
      if (ae && ae !== document.body) {
        var tag = ae.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') { return; }
        if (ae.isContentEditable) { return; }
      }
      e.preventDefault();
      input.focus();
      input.select();
    });
  }

  // ---------- Line anchors (detail pages) -------------------------------
  // Hooks: <button class="line-anchor" data-line="42"> inside td.line-num,
  // + <div id="cov-copy-toast" role="status" aria-live="polite">.
  // Clicking the button: scrolls to the row via hash, copies the
  // canonical link to the clipboard, flashes a toast.
  function initLineAnchors() {
    var anchors = document.querySelectorAll('button.line-anchor[data-line]');
    if (anchors.length === 0) { return; }
    var toast = document.getElementById('cov-copy-toast');
    var toastTimer = 0;

    function flashToast(message) {
      if (!toast) { return; }
      // The screen-reader announcement is triggered by the textContent
      // assignment below, not by the `visible` class toggle. The class
      // controls only the visual fade-in; do not switch to display:none
      // for the hide step or the live-region observer will detach.
      toast.textContent = message;
      toast.classList.add('visible');
      if (toastTimer) { clearTimeout(toastTimer); }
      toastTimer = setTimeout(function () {
        toast.classList.remove('visible');
        toast.textContent = '';
      }, 1500);
    }

    function copyAndAnchor(line) {
      var hash = '#L' + line;
      history.replaceState(null, '', hash);
      var url = location.href;
      var ok = false;
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(url).then(function () {
          flashToast('Link to line ' + line + ' copied');
        }).catch(function () {
          flashToast('Linked to line ' + line);
        });
        ok = true;
      }
      if (!ok) {
        // Older browsers / file:// origins where clipboard API is
        // unavailable: still anchor the URL so the user can copy it
        // from the address bar.
        flashToast('Linked to line ' + line);
      }
    }

    anchors.forEach(function (btn) {
      btn.addEventListener('click', function () {
        var line = btn.getAttribute('data-line');
        if (line) { copyAndAnchor(line); }
      });
    });
  }

  // ---------- "Next uncovered" jump button ------------------------------
  // Hook: <button id="cov-next-uncovered">. Cycles through
  // tr.line.miss and tr.line.partial in document order; wraps at end.
  // Disabled when the page has no uncovered rows.
  function initNextUncovered() {
    var btn = document.getElementById('cov-next-uncovered');
    if (!btn) { return; }
    var rows = document.querySelectorAll('table.source tr.line.miss, table.source tr.line.partial');
    if (rows.length === 0) {
      btn.disabled = true;
      btn.setAttribute('aria-label', 'No uncovered lines on this page');
      btn.textContent = 'No uncovered lines';
      return;
    }
    btn.disabled = false;
    var cursor = -1;
    btn.addEventListener('click', function () {
      cursor = (cursor + 1) % rows.length;
      var target = rows[cursor];
      var line = target.getAttribute('id') || '';
      if (line) { history.replaceState(null, '', '#' + line); }
      target.scrollIntoView({ block: 'center', behavior: 'auto' });
      var inner = target.querySelector('button.line-anchor');
      if (inner) { inner.focus(); }
    });
  }

  // ---------- Boot ------------------------------------------------------
  function boot() {
    buildThemeToggle();
    initSortable();
    initFilter();
    initLineAnchors();
    initNextUncovered();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
}());
