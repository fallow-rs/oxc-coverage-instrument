// Minimal TypeScript module exercising:
//   - typed function declaration with parameter and return annotations
//   - interface (type-only, no counter)
//   - type alias (type-only, no counter)
//   - if-else branch
//   - early return
//
// Coverage must point at THESE line numbers (1-based), not at any intermediate
// JS produced by a Babel/tsc transform.

export interface Operand {
    value: number;
}

export type Op = 'add' | 'sub';

export function compute(a: Operand, b: Operand, op: Op): number {
    if (op === 'add') {
        return a.value + b.value;
    }
    return a.value - b.value;
}
