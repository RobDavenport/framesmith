import { describe, expect, it } from 'vitest';
import type { Writable } from 'svelte/store';
import { InputBuffer, InputManager, MoveResolver, DummyController } from '$lib/training';
import { DummyState, type CharacterState, type FrameResult, type HitResult, type PushSeparation, type TrainingSnapshot } from '$lib/training/TrainingSession';
import type { MoveDefinition } from '$lib/training/MoveResolver';
import type { Character, State } from '$lib/types';
import { TrainingLoop, type TrainingLoopConfig } from './TrainingLoop';

function getStoreValue<T>(store: Writable<T>): T {
  let value: T | undefined;
  const unsubscribe = store.subscribe(nextValue => {
    value = nextValue;
  });
  unsubscribe();
  return value as T;
}

function makeCharacterState(overrides: Partial<CharacterState> = {}): CharacterState {
  return {
    current_state: 0,
    frame: 0,
    instance_duration: 0,
    hit_confirmed: false,
    block_confirmed: false,
    resources: [0, 0, 0, 0, 0, 0, 0, 0],
    ...overrides,
  };
}

function cloneCharacterState(state: CharacterState): CharacterState {
  return {
    ...state,
    resources: [...state.resources],
  };
}

class MockTrainingSession {
  player = makeCharacterState();
  dummy = makeCharacterState();
  playerX = 350;
  playerY = 0;
  dummyX = 450;
  dummyY = 0;
  nextHits: HitResult[] = [];
  nextPushSeparation: PushSeparation | undefined;
  availableCancelTargets: number[] = [];
  lastPlayerInput: number | null = null;
  lastDummyState: DummyState | null = null;

  tick(playerInput: number, dummyState: DummyState): FrameResult {
    this.lastPlayerInput = playerInput;
    this.lastDummyState = dummyState;
    this.player = makeCharacterState({ ...this.player, frame: this.player.frame + 1 });
    this.dummy = makeCharacterState({ ...this.dummy, frame: this.dummy.frame + 1 });

    const hits = this.nextHits.map(hit => ({ ...hit }));
    const pushSeparation = this.nextPushSeparation
      ? { ...this.nextPushSeparation }
      : undefined;
    this.nextHits = [];
    this.nextPushSeparation = undefined;

    return {
      player: cloneCharacterState(this.player),
      dummy: cloneCharacterState(this.dummy),
      hits,
      push_separation: pushSeparation,
    };
  }

  playerState(): CharacterState {
    return cloneCharacterState(this.player);
  }

  dummyState(): CharacterState {
    return cloneCharacterState(this.dummy);
  }

  availableCancels(): number[] {
    return [...this.availableCancelTargets];
  }

  reset(): void {
    this.player = makeCharacterState();
    this.dummy = makeCharacterState();
    this.playerX = 350;
    this.playerY = 0;
    this.dummyX = 450;
    this.dummyY = 0;
  }

  setPositions(playerX: number, playerY: number, dummyX: number, dummyY: number): void {
    this.playerX = playerX;
    this.playerY = playerY;
    this.dummyX = dummyX;
    this.dummyY = dummyY;
  }

  snapshot(): TrainingSnapshot {
    return {
      player: cloneCharacterState(this.player),
      dummy: cloneCharacterState(this.dummy),
      player_x: this.playerX,
      player_y: this.playerY,
      dummy_x: this.dummyX,
      dummy_y: this.dummyY,
    };
  }

  restore(snapshot: TrainingSnapshot): void {
    this.player = cloneCharacterState(snapshot.player);
    this.dummy = cloneCharacterState(snapshot.dummy);
    this.playerX = snapshot.player_x;
    this.playerY = snapshot.player_y;
    this.dummyX = snapshot.dummy_x;
    this.dummyY = snapshot.dummy_y;
  }
}

function createHit(overrides: Partial<HitResult> = {}): HitResult {
  return {
    attacker_move: 0,
    window_index: 0,
    blocked: false,
    damage: 25,
    chip_damage: 4,
    hitstun: 18,
    blockstun: 12,
    hitstop: 6,
    guard: 1,
    hit_pushback: 10,
    block_pushback: 6,
    ...overrides,
  };
}

function createLoop(options: {
  character?: Character;
  moves?: State[];
  dummyController?: DummyController;
  moveResolver?: MoveResolver;
} = {}): { loop: TrainingLoop; session: MockTrainingSession; inputManager: InputManager } {
  const session = new MockTrainingSession();
  const character = options.character ?? {
    id: 'test',
    name: 'Test Character',
    properties: {
      health: 100,
    },
    resources: [],
  } as Character;
  const inputManager = new InputManager({
    directions: {
      up: 'KeyW',
      down: 'KeyS',
      left: 'KeyA',
      right: 'KeyD',
    },
    buttons: {
      L: 'KeyU',
      M: 'KeyI',
      H: 'KeyO',
      P: 'KeyJ',
      K: 'KeyK',
      S: 'KeyL',
      T: 'KeyP',
    },
  });

  const loop = new TrainingLoop({
    session: session as unknown as TrainingLoopConfig['session'],
    inputManager,
    inputBuffer: new InputBuffer(),
    moveResolver: options.moveResolver ?? new MoveResolver({ moves: [], moveNameToIndex: new Map() }),
    dummyController: options.dummyController ?? new DummyController(),
    character,
    moves: options.moves ?? [],
  });

  return { loop, session, inputManager };
}

describe('TrainingLoop rewind history', () => {
  it('steps back to the previous loop and WASM session state', () => {
    const { loop, session } = createLoop();

    loop.state.update(state => ({
      ...state,
      playerX: 312,
      dummyX: 488,
    }));
    loop.stepForward();
    expect(getStoreValue(loop.state).frameCount).toBe(1);
    expect(session.playerState().frame).toBe(1);
    expect(session.playerX).toBe(312);
    expect(session.dummyX).toBe(488);

    session.setPositions(999, 0, 999, 0);
    loop.stepBack();
    const rewound = getStoreValue(loop.state);

    expect(rewound.isPlaying).toBe(false);
    expect(rewound.frameCount).toBe(0);
    expect(rewound.playerState?.frame).toBe(0);
    expect(rewound.playerX).toBe(312);
    expect(rewound.dummyX).toBe(488);
    expect(session.playerState().frame).toBe(0);
    expect(session.playerX).toBe(312);
    expect(session.dummyX).toBe(488);

    loop.stepForward();
    expect(getStoreValue(loop.state).frameCount).toBe(1);
    expect(session.playerState().frame).toBe(1);
  });

  it('keeps the current state when stepping back without history', () => {
    const { loop, session } = createLoop();

    loop.stepBack();
    const state = getStoreValue(loop.state);

    expect(state.isPlaying).toBe(false);
    expect(state.frameCount).toBe(0);
    expect(session.playerState().frame).toBe(0);
  });

  it('clears rewind history when resetting training state', () => {
    const { loop, session } = createLoop();

    loop.stepForward();
    loop.stepForward();
    loop.resetHealth();

    const reset = getStoreValue(loop.state);
    expect(reset.frameCount).toBe(0);
    expect(reset.inputHistory).toEqual([]);
    expect(reset.dummyHealth).toBe(reset.maxHealth);
    expect(session.playerState().frame).toBe(0);

    loop.stepBack();
    const afterBack = getStoreValue(loop.state);
    expect(afterBack.frameCount).toBe(0);
    expect(session.playerState().frame).toBe(0);
  });
});

describe('TrainingLoop behavior coverage', () => {
  it('passes authored dummy stance choices to the WASM session', () => {
    const dummyController = new DummyController();
    dummyController.setState('crouch');
    const { loop, session } = createLoop({ dummyController });

    loop.stepForward();

    expect(session.lastDummyState).toBe(DummyState.Crouch);
  });

  it('applies full damage on hit and chip damage on block', () => {
    const { loop, session } = createLoop();

    session.nextHits = [createHit({ damage: 25, chip_damage: 3, blocked: false })];
    loop.stepForward();

    let state = getStoreValue(loop.state);
    expect(state.dummyHealth).toBe(75);
    expect(state.comboHits).toBe(1);
    expect(state.comboDamage).toBe(25);

    session.nextHits = [createHit({ damage: 50, chip_damage: 5, blocked: true })];
    loop.stepForward();

    state = getStoreValue(loop.state);
    expect(state.dummyHealth).toBe(70);
    expect(state.comboHits).toBe(2);
    expect(state.comboDamage).toBe(30);
  });

  it('resets combo tracking after the quiet reset window', () => {
    const { loop, session } = createLoop();

    session.nextHits = [createHit()];
    loop.stepForward();
    expect(getStoreValue(loop.state).comboHits).toBe(1);

    for (let i = 0; i < 60; i++) {
      loop.stepForward();
    }

    const state = getStoreValue(loop.state);
    expect(state.comboHits).toBe(0);
    expect(state.comboDamage).toBe(0);
  });

  it('applies runtime push separation and authored movement', () => {
    const moves = [{
      input: '66',
      name: 'Forward Dash',
      type: 'movement',
      startup: 1,
      active: 1,
      recovery: 1,
      total: 3,
      movement: { type: 'dash', distance: 30, direction: 'forward' },
    }] as unknown as State[];
    const { loop, session } = createLoop({ moves });
    session.nextPushSeparation = { player_dx: -4, dummy_dx: 6 };

    loop.stepForward();

    const state = getStoreValue(loop.state);
    expect(state.playerX).toBe(356);
    expect(state.dummyX).toBe(456);
  });

  it('resolves throw inputs through the regular input path', () => {
    const moves: MoveDefinition[] = [{ name: '5T', input: { type: 'simple', direction: 5, button: 'T' }, priority: 50 }];
    const moveResolver = new MoveResolver({ moves, moveNameToIndex: new Map([['5T', 0]]) });
    const { loop, session, inputManager } = createLoop({ moveResolver });

    inputManager.handleKeyDown('KeyP');
    loop.stepForward();

    expect(session.lastPlayerInput).toBe(0);
  });
});
