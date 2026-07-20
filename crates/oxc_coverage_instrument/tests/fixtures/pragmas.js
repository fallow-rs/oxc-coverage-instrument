// Pragma test: selective ignoring

function alwaysCounted() {
  return 1;
}

/* istanbul ignore next */
function ignoredFunction() {
  return 2;
}

function withIgnoredBranch(x) {
  /* istanbul ignore if */
  if (x < 0) {
    throw new Error('negative');
  }
  return x * 2;
}

function withIgnoredElse(x) {
  /* istanbul ignore else */
  if (x > 0) {
    return 'positive';
  } else {
    return 'non-positive';
  }
}

/* v8 ignore next */
function v8Ignored() {
  return 3;
}

/* c8 ignore next */
function c8Ignored() {
  return 4;
}

const counted = () => alwaysCounted() + withIgnoredBranch(1) + withIgnoredElse(1);
