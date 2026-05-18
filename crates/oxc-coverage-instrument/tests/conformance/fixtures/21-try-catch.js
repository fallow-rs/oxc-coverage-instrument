function f(x) {
  try {
    return JSON.parse(x);
  } catch (e) {
    return null;
  } finally {
    void 0;
  }
}
