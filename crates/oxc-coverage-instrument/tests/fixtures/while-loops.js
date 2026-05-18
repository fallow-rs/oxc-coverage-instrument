// Tests for while and do-while loop branch coverage

function readStream(stream) {
  let chunk;
  const chunks = [];
  while ((chunk = stream.read()) !== null) {
    chunks.push(chunk);
    if (chunks.length > 1000) {
      throw new Error('Too many chunks');
    }
  }
  return Buffer.concat(chunks);
}

function retry(fn, maxAttempts = 3) {
  let attempts = 0;
  let lastError;
  do {
    try {
      return fn();
    } catch (err) {
      lastError = err;
      attempts++;
    }
  } while (attempts < maxAttempts);
  throw lastError;
}

function pollUntilReady(checkFn, timeout = 5000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const result = checkFn();
    if (result?.ready) return result;
  }
  return null;
}

async function consumeAsyncIterator(iter) {
  const results = [];
  for await (const item of iter) {
    if (item.done) break;
    results.push(item.value);
  }
  return results;
}

module.exports = { readStream, retry, pollUntilReady, consumeAsyncIterator };
