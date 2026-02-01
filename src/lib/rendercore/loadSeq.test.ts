import { describe, it, expect } from 'vitest';

import { createLoadSeq } from './loadSeq';

describe('rendercore loadSeq', () => {
  it('marks only the latest seq as current', () => {
    const seq = createLoadSeq();

    const a = seq.next();
    expect(seq.isCurrent(a)).toBe(true);

    const b = seq.next();
    expect(seq.isCurrent(a)).toBe(false);
    expect(seq.isCurrent(b)).toBe(true);
  });
});
