/**
 * Tests for DummyController - manages training mode dummy behavior.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { DummyController, type DummyConfig, type DummyState } from './DummyController';
import { DummyState as WasmDummyState } from './TrainingSession';

describe('DummyController', () => {
  let controller: DummyController;

  beforeEach(() => {
    controller = new DummyController();
  });

  describe('default configuration', () => {
    it('should default to stand state', () => {
      expect(controller.config.state).toBe('stand');
    });

    it('should default to neutral recovery', () => {
      expect(controller.config.recovery).toBe('neutral');
    });

    it('should default counter_on_hit to false', () => {
      expect(controller.config.counter_on_hit).toBe(false);
    });

    it('should have no reversal_move by default', () => {
      expect(controller.config.reversal_move).toBeUndefined();
    });
  });

  describe('state configuration', () => {
    it('should allow setting stand state', () => {
      controller.setState('stand');
      expect(controller.config.state).toBe('stand');
    });

    it('should allow setting crouch state', () => {
      controller.setState('crouch');
      expect(controller.config.state).toBe('crouch');
    });

    it('should allow setting jump state', () => {
      controller.setState('jump');
      expect(controller.config.state).toBe('jump');
    });

    it('should allow setting block_stand state', () => {
      controller.setState('block_stand');
      expect(controller.config.state).toBe('block_stand');
    });

    it('should allow setting block_crouch state', () => {
      controller.setState('block_crouch');
      expect(controller.config.state).toBe('block_crouch');
    });

    it('should allow setting block_auto state', () => {
      controller.setState('block_auto');
      expect(controller.config.state).toBe('block_auto');
    });
  });

  describe('recovery configuration', () => {
    it('should allow setting neutral recovery', () => {
      controller.setRecovery('neutral');
      expect(controller.config.recovery).toBe('neutral');
    });

    it('should allow setting reversal recovery', () => {
      controller.setRecovery('reversal');
      expect(controller.config.recovery).toBe('reversal');
    });

    it('should allow setting reversal move', () => {
      controller.setReversalMove('5L');
      expect(controller.config.reversal_move).toBe('5L');
    });

    it('should allow clearing reversal move', () => {
      controller.setReversalMove('5L');
      controller.setReversalMove(undefined);
      expect(controller.config.reversal_move).toBeUndefined();
    });
  });

  describe('counter on hit configuration', () => {
    it('should allow enabling counter on hit', () => {
      controller.setCounterOnHit(true);
      expect(controller.config.counter_on_hit).toBe(true);
    });

    it('should allow disabling counter on hit', () => {
      controller.setCounterOnHit(true);
      controller.setCounterOnHit(false);
      expect(controller.config.counter_on_hit).toBe(false);
    });
  });

  describe('WASM state conversion', () => {
    it('should convert stand to WASM DummyState.Stand', () => {
      controller.setState('stand');
      expect(controller.getWasmState()).toBe(WasmDummyState.Stand);
    });

    it('should convert crouch to WASM DummyState.Crouch', () => {
      controller.setState('crouch');
      expect(controller.getWasmState()).toBe(WasmDummyState.Crouch);
    });

    it('should convert jump to WASM DummyState.Jump', () => {
      controller.setState('jump');
      expect(controller.getWasmState()).toBe(WasmDummyState.Jump);
    });

    it('should convert block_stand to WASM DummyState.BlockStand', () => {
      controller.setState('block_stand');
      expect(controller.getWasmState()).toBe(WasmDummyState.BlockStand);
    });

    it('should convert block_crouch to WASM DummyState.BlockCrouch', () => {
      controller.setState('block_crouch');
      expect(controller.getWasmState()).toBe(WasmDummyState.BlockCrouch);
    });

    it('should convert block_auto to WASM DummyState.BlockAuto', () => {
      controller.setState('block_auto');
      expect(controller.getWasmState()).toBe(WasmDummyState.BlockAuto);
    });
  });

  describe('full config update', () => {
    it('should allow setting full config', () => {
      const newConfig: DummyConfig = {
        state: 'block_auto',
        recovery: 'reversal',
        reversal_move: '236P',
        counter_on_hit: true,
      };
      controller.setConfig(newConfig);

      expect(controller.config.state).toBe('block_auto');
      expect(controller.config.recovery).toBe('reversal');
      expect(controller.config.reversal_move).toBe('236P');
      expect(controller.config.counter_on_hit).toBe(true);
    });
  });

  describe('config immutability', () => {
    it('should return a copy of the config', () => {
      const config1 = controller.config;
      controller.setState('crouch');
      const config2 = controller.config;

      // The returned config should be independent
      expect(config1.state).toBe('stand');
      expect(config2.state).toBe('crouch');
    });
  });

  describe('reset', () => {
    it('should reset to default config', () => {
      controller.setState('block_auto');
      controller.setRecovery('reversal');
      controller.setReversalMove('5H');
      controller.setCounterOnHit(true);

      controller.reset();

      expect(controller.config.state).toBe('stand');
      expect(controller.config.recovery).toBe('neutral');
      expect(controller.config.reversal_move).toBeUndefined();
      expect(controller.config.counter_on_hit).toBe(false);
    });
  });
});
