export const fixtureFilename = 'worker-fixture.js';

export const fixtureSource = Array.from({ length: 100 }, (_, index) => {
  const n = index + 1;
  const op = n % 2 === 0 ? '>=' : '<';
  return `export function branch${n}(value) { return value ${op} ${n} ? ${n} : -${n}; }`;
}).join('\n');
