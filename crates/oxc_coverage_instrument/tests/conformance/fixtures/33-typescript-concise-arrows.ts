// Concise arrow bodies that are nothing but a TypeScript-only wrapper. The
// strip pass rewrites every one of these bodies away, so the span the coverage
// map records has to be the one the source wrote, not what survives stripping.
// 32-typescript-stripped.ts holds no arrow, so without this fixture the
// istanbul comparison never reaches the concise-arrow path under stripping.

const asChain = (x: number) => x as unknown as string;
const satisfiesBody = (x: number) => ({ a: x }) satisfies { a: number };
const angleAssert = (x: unknown) => <string>x;
const nonNull = (x?: number) => x!;
const instantiate = (f: <T>(v: T) => T) => f<number>;

// `instantiate` needs a generic function to instantiate.
const identity = <T,>(v: T): T => v;

globalThis.typescriptConciseArrows = [
  asChain(1),
  satisfiesBody(2).a,
  angleAssert('three'),
  nonNull(5),
  instantiate(identity)(6),
  identity(7),
];
