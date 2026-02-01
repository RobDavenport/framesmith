import { describe, it, expect } from 'vitest';

import { mergeActorStatus } from './status';

describe('rendercore status', () => {
  it('does not overwrite an update error with a later status read', () => {
    const merged = mergeActorStatus(
      { loading: false, error: null },
      'boom',
      { loading: false, error: null }
    );

    expect(merged.error).toBe('boom');
  });
});
