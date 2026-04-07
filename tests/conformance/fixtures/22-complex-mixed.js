function f(items, threshold) {
  let result = 0;
  for (const item of items) {
    if (item > threshold) {
      result += item > 100 ? 100 : item;
    } else {
      const fallback = item ?? 0;
      result += fallback && fallback > 0 ? fallback : 1;
    }
  }
  return result;
}
