import { describe, it, expect } from 'vitest';

import { shouldStartLoad } from './loadDecision';

describe('rendercore loadDecision', () => {
  it('does not restart an in-flight load for the same path', () => {
    expect(
      shouldStartLoad({
        requestedPath: 'a.png',
        loadedPath: null,
        inflightPath: 'a.png',
      })
    ).toBe(false);
  });

  it('does not start a load if already loaded', () => {
    expect(
      shouldStartLoad({
        requestedPath: 'a.png',
        loadedPath: 'a.png',
        inflightPath: null,
      })
    ).toBe(false);
  });

  it('starts a new load when the requested path changes', () => {
    expect(
      shouldStartLoad({
        requestedPath: 'b.png',
        loadedPath: 'a.png',
        inflightPath: 'a.png',
      })
    ).toBe(true);
  });
});
