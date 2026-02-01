import type { CharacterAssets } from '$lib/types';

export type PickAnimationKeyResult = {
  key: string | null;
  note: string | null;
};

export function pickAnimationKey(
  assets: Pick<CharacterAssets, 'animations'>,
  preferred: string
): PickAnimationKeyResult {
  const pref = (preferred ?? '').trim();
  if (pref && assets.animations[pref]) return { key: pref, note: null };

  const missingNote = pref
    ? `Animation key not found: '${pref}'`
    : 'State.animation is empty';

  if (assets.animations['idle']) {
    return { key: 'idle', note: `${missingNote} (fallback: 'idle')` };
  }

  const first = Object.keys(assets.animations)[0] ?? null;
  if (first) {
    return { key: first, note: `${missingNote} (fallback: '${first}')` };
  }

  return { key: null, note: `${missingNote}; no animations available` };
}
