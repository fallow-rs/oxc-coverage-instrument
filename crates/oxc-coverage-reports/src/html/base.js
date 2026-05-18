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

  // ---------- Boot ------------------------------------------------------
  function boot() {
    buildThemeToggle();
    initSortable();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
}());
