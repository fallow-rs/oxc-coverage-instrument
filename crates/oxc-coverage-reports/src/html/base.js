/* oxc-coverage-instrument HTML reporter, PR G2 polish layer.
 *
 * Self-contained, no external dependencies, no network access. Provides:
 *   1. theme toggle (auto / light / dark, persisted to localStorage),
 *   2. sortable index tables (click or keyboard, aria-sort updates),
 *   3. lightweight JS / TS syntax highlighting on detail-page sources.
 *
 * The page is fully usable without JS: missing JS just means no sort, no
 * explicit toggle (prefers-color-scheme still works), and no syntax
 * highlighting. Every feature is progressive enhancement.
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

  // ---------- Source prettify -------------------------------------------
  // Tiny tokenizer covering JS / TS / JSX surface adequate for an
  // unminified source view. Aims for "GitHub-style readable", not a
  // full language parser. Emits <span class="tok-*"> nodes via the DOM.
  var KEYWORDS = (
    'abstract,any,as,async,await,boolean,break,case,catch,class,const,' +
    'constructor,continue,debugger,declare,default,delete,do,else,enum,' +
    'export,extends,false,finally,for,from,function,get,if,implements,' +
    'import,in,infer,instanceof,interface,is,keyof,let,module,namespace,' +
    'never,new,null,number,object,of,override,package,private,protected,' +
    'public,readonly,require,return,satisfies,set,static,string,super,' +
    'switch,symbol,this,throw,true,try,type,typeof,undefined,unique,unknown,' +
    'var,void,while,with,yield'
  ).split(',');
  var KEYWORD_SET = {};
  for (var ki = 0; ki < KEYWORDS.length; ki++) { KEYWORD_SET[KEYWORDS[ki]] = true; }

  var BUILTINS = (
    'Array,ArrayBuffer,Boolean,Buffer,console,DataView,Date,document,Error,' +
    'EvalError,Function,globalThis,Infinity,Intl,JSON,Map,Math,NaN,' +
    'Number,Object,process,Promise,Proxy,RangeError,ReferenceError,Reflect,' +
    'RegExp,Set,String,Symbol,SyntaxError,TypeError,URIError,WeakMap,WeakSet,window'
  ).split(',');
  var BUILTIN_SET = {};
  for (var bi = 0; bi < BUILTINS.length; bi++) { BUILTIN_SET[BUILTINS[bi]] = true; }

  function isIdentStart(ch) {
    return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_' || ch === '$';
  }
  function isIdentCont(ch) {
    return isIdentStart(ch) || (ch >= '0' && ch <= '9');
  }
  function isDigit(ch) { return ch >= '0' && ch <= '9'; }
  function isHexCont(ch) {
    return isDigit(ch) || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
      || ch === '_' || ch === '.';
  }

  function tokenize(src) {
    var tokens = [];
    var i = 0;
    var n = src.length;
    while (i < n) {
      var ch = src.charAt(i);

      // line comment
      if (ch === '/' && src.charAt(i + 1) === '/') {
        var j = i + 2;
        while (j < n && src.charAt(j) !== '\n') { j++; }
        tokens.push(['c', src.slice(i, j)]);
        i = j;
        continue;
      }

      // block comment
      if (ch === '/' && src.charAt(i + 1) === '*') {
        var k = i + 2;
        while (k + 1 < n && !(src.charAt(k) === '*' && src.charAt(k + 1) === '/')) {
          k++;
        }
        k = k + 1 < n ? k + 2 : n;
        tokens.push(['c', src.slice(i, k)]);
        i = k;
        continue;
      }

      // string / template
      if (ch === '"' || ch === '\'' || ch === '`') {
        var quote = ch;
        var m = i + 1;
        while (m < n) {
          var cc = src.charAt(m);
          if (cc === '\\' && m + 1 < n) { m += 2; continue; }
          if (cc === quote) { m++; break; }
          if (cc === '\n' && quote !== '`') { break; }
          m++;
        }
        tokens.push(['s', src.slice(i, m)]);
        i = m;
        continue;
      }

      // number (decimal / hex / float). Heuristic; good enough.
      if (isDigit(ch) || (ch === '.' && isDigit(src.charAt(i + 1)))) {
        var p = i + 1;
        while (p < n && isHexCont(src.charAt(p))) { p++; }
        if (p < n && (src.charAt(p) === 'n' || src.charAt(p) === 'e' || src.charAt(p) === 'E')) {
          p++;
          if (p < n && (src.charAt(p) === '+' || src.charAt(p) === '-')) { p++; }
          while (p < n && isDigit(src.charAt(p))) { p++; }
        }
        tokens.push(['n', src.slice(i, p)]);
        i = p;
        continue;
      }

      // identifier / keyword / builtin
      if (isIdentStart(ch)) {
        var q = i + 1;
        while (q < n && isIdentCont(src.charAt(q))) { q++; }
        var word = src.slice(i, q);
        if (KEYWORD_SET[word]) { tokens.push(['k', word]); }
        else if (BUILTIN_SET[word]) { tokens.push(['b', word]); }
        else { tokens.push(['i', word]); }
        i = q;
        continue;
      }

      // punctuation (single char is enough for highlight purposes)
      if ('+-*/%=<>!&|^~?:,;.(){}[]'.indexOf(ch) !== -1) {
        tokens.push(['p', ch]);
        i++;
        continue;
      }

      // anything else (whitespace / unicode / etc): pass through plain
      tokens.push(['t', ch]);
      i++;
    }
    return tokens;
  }

  function renderTokensInto(pre, tokens) {
    while (pre.firstChild) { pre.removeChild(pre.firstChild); }
    for (var i = 0; i < tokens.length; i++) {
      var kind = tokens[i][0];
      var text = tokens[i][1];
      if (kind === 't' || kind === 'i') {
        pre.appendChild(document.createTextNode(text));
      } else {
        var span = document.createElement('span');
        span.className = 'tok-' + kind;
        span.textContent = text;
        pre.appendChild(span);
      }
    }
  }

  function initPrettify() {
    var pres = document.querySelectorAll('table.source td.src pre');
    for (var i = 0; i < pres.length; i++) {
      var pre = pres[i];
      var text = pre.textContent;
      if (!text || text === '(source unavailable)') { continue; }
      renderTokensInto(pre, tokenize(text));
    }
  }

  // ---------- Boot ------------------------------------------------------
  function boot() {
    buildThemeToggle();
    initSortable();
    initPrettify();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
}());
