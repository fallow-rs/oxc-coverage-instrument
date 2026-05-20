import { describe, expect, it } from 'vitest';
import { compute, type Operand } from './math.ts';

describe('compute', () => {
    const a: Operand = { value: 5 };
    const b: Operand = { value: 3 };

    it('adds operands', () => {
        expect(compute(a, b, 'add')).toBe(8);
    });

    it('subtracts operands', () => {
        expect(compute(a, b, 'sub')).toBe(2);
    });
});
