function outer(x) {
  function inner(y) {
    return y * 2;
  }
  return inner(x) + 1;
}
