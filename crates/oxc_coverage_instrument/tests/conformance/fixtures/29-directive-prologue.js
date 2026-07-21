"use strict";

function strictThis() {
  return this;
}

globalThis.directiveResult = strictThis();
