/**
 * Tests for TrainingSync module.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  TrainingSync,
  createMainWindowSync,
  createDetachedWindowSync,
  type TrainingSyncMessage,
} from './TrainingSync';
import type { CharacterData } from '$lib/types';

// Mock CharacterData for testing
const mockCharacterData: CharacterData = {
  character: {
    id: 'test-char',
    name: 'Test Character',
    archetype: 'Test',
    health: 10000,
    walk_speed: 4.0,
    back_walk_speed: 3.0,
    jump_height: 120,
    jump_duration: 45,
    dash_distance: 80,
    dash_duration: 18,
  },
  moves: [],
  cancel_table: {
    chains: {},
    special_cancels: [],
    super_cancels: [],
    jump_cancels: [],
  },
};

// Mock BroadcastChannel
class MockBroadcastChannel {
  static instances: MockBroadcastChannel[] = [];
  name: string;
  onmessage: ((event: MessageEvent) => void) | null = null;
  closed = false;

  constructor(name: string) {
    this.name = name;
    MockBroadcastChannel.instances.push(this);
  }

  postMessage(message: TrainingSyncMessage): void {
    if (this.closed) return;

    // Broadcast to all other instances with the same name
    for (const instance of MockBroadcastChannel.instances) {
      if (instance !== this && instance.name === this.name && !instance.closed) {
        if (instance.onmessage) {
          instance.onmessage(new MessageEvent('message', { data: message }));
        }
      }
    }
  }

  close(): void {
    this.closed = true;
    const index = MockBroadcastChannel.instances.indexOf(this);
    if (index > -1) {
      MockBroadcastChannel.instances.splice(index, 1);
    }
  }

  static reset(): void {
    MockBroadcastChannel.instances = [];
  }
}

describe('TrainingSync', () => {
  beforeEach(() => {
    MockBroadcastChannel.reset();
    // @ts-ignore - Replace global BroadcastChannel
    global.BroadcastChannel = MockBroadcastChannel;
  });

  afterEach(() => {
    MockBroadcastChannel.reset();
  });

  describe('TrainingSync class', () => {
    it('should create a channel and handle messages', () => {
      const onCharacterChange = vi.fn();
      const sync = new TrainingSync({ onCharacterChange });

      // Create another sync to send a message
      const sender = new TrainingSync();
      sender.sendCharacterChange(mockCharacterData);

      expect(onCharacterChange).toHaveBeenCalledWith(mockCharacterData);

      sync.destroy();
      sender.destroy();
    });

    it('should send character save events', () => {
      const onCharacterSave = vi.fn();
      const receiver = new TrainingSync({ onCharacterSave });
      const sender = new TrainingSync();

      sender.sendCharacterSave(mockCharacterData);

      expect(onCharacterSave).toHaveBeenCalledWith(mockCharacterData);

      receiver.destroy();
      sender.destroy();
    });

    it('should send project path events', () => {
      const onProjectPath = vi.fn();
      const receiver = new TrainingSync({ onProjectPath });
      const sender = new TrainingSync();

      sender.sendProjectPath('/test/path');

      expect(onProjectPath).toHaveBeenCalledWith('/test/path');

      sender.sendProjectPath(null);

      expect(onProjectPath).toHaveBeenCalledWith(null);

      receiver.destroy();
      sender.destroy();
    });

    it('should send sync request events', () => {
      const onSyncRequest = vi.fn();
      const receiver = new TrainingSync({ onSyncRequest });
      const sender = new TrainingSync();

      sender.requestSync();

      expect(onSyncRequest).toHaveBeenCalled();

      receiver.destroy();
      sender.destroy();
    });

    it('should respond to ping with pong', () => {
      const sender = new TrainingSync();
      const receiver = new TrainingSync();

      // Capture pong response
      const pongReceived = vi.fn();
      const channel = sender['channel'];
      const originalOnMessage = channel?.onmessage;
      if (channel) {
        channel.onmessage = (event: MessageEvent) => {
          if (event.data.type === 'pong') {
            pongReceived();
          }
          originalOnMessage?.call(channel, event);
        };
      }

      sender.ping();

      // The receiver should send a pong back
      expect(pongReceived).toHaveBeenCalled();

      sender.destroy();
      receiver.destroy();
    });

    it('should allow updating callbacks', () => {
      const sync = new TrainingSync();
      const onCharacterChange = vi.fn();

      sync.setCallbacks({ onCharacterChange });

      const sender = new TrainingSync();
      sender.sendCharacterChange(mockCharacterData);

      expect(onCharacterChange).toHaveBeenCalledWith(mockCharacterData);

      sync.destroy();
      sender.destroy();
    });

    it('should not send messages after destroy', () => {
      const onCharacterChange = vi.fn();
      const receiver = new TrainingSync({ onCharacterChange });
      const sender = new TrainingSync();

      sender.destroy();
      sender.sendCharacterChange(mockCharacterData);

      expect(onCharacterChange).not.toHaveBeenCalled();

      receiver.destroy();
    });

    it('should not handle messages after destroy', () => {
      const onCharacterChange = vi.fn();
      const receiver = new TrainingSync({ onCharacterChange });
      const sender = new TrainingSync();

      receiver.destroy();
      sender.sendCharacterChange(mockCharacterData);

      expect(onCharacterChange).not.toHaveBeenCalled();

      sender.destroy();
    });
  });

  describe('createMainWindowSync', () => {
    it('should respond to sync requests with current state', () => {
      const getCharacter = vi.fn(() => mockCharacterData);
      const getProjectPath = vi.fn(() => '/test/project');

      const mainSync = createMainWindowSync(getCharacter, getProjectPath);

      // Create a detached window that will receive the sync
      const onCharacterChange = vi.fn();
      const onProjectPath = vi.fn();
      const detachedSync = new TrainingSync({
        onCharacterChange,
        onProjectPath,
      });

      // Trigger sync request from detached
      detachedSync.requestSync();

      expect(onCharacterChange).toHaveBeenCalledWith(mockCharacterData);
      expect(onProjectPath).toHaveBeenCalledWith('/test/project');

      mainSync.destroy();
      detachedSync.destroy();
    });

    it('should not send character if none is loaded', () => {
      const getCharacter = vi.fn(() => null);
      const getProjectPath = vi.fn(() => '/test/project');

      const mainSync = createMainWindowSync(getCharacter, getProjectPath);

      const onCharacterChange = vi.fn();
      const onProjectPath = vi.fn();
      const detachedSync = new TrainingSync({
        onCharacterChange,
        onProjectPath,
      });

      detachedSync.requestSync();

      expect(onCharacterChange).not.toHaveBeenCalled();
      expect(onProjectPath).toHaveBeenCalledWith('/test/project');

      mainSync.destroy();
      detachedSync.destroy();
    });
  });

  describe('createDetachedWindowSync', () => {
    it('should handle updates in live mode', () => {
      const onCharacterUpdate = vi.fn();
      const onProjectPath = vi.fn();

      const detachedSync = createDetachedWindowSync(
        onCharacterUpdate,
        onProjectPath,
        'live'
      );

      const mainSync = new TrainingSync();

      // Both change and save should trigger update in live mode
      mainSync.sendCharacterChange(mockCharacterData);
      expect(onCharacterUpdate).toHaveBeenCalledTimes(1);

      mainSync.sendCharacterSave(mockCharacterData);
      expect(onCharacterUpdate).toHaveBeenCalledTimes(2);

      detachedSync.destroy();
      mainSync.destroy();
    });

    it('should only handle save events in on-save mode', () => {
      const onCharacterUpdate = vi.fn();
      const onProjectPath = vi.fn();

      const detachedSync = createDetachedWindowSync(
        onCharacterUpdate,
        onProjectPath,
        'on-save'
      );

      const mainSync = new TrainingSync();

      // Change should NOT trigger update in on-save mode
      mainSync.sendCharacterChange(mockCharacterData);
      expect(onCharacterUpdate).not.toHaveBeenCalled();

      // Save SHOULD trigger update
      mainSync.sendCharacterSave(mockCharacterData);
      expect(onCharacterUpdate).toHaveBeenCalledTimes(1);

      detachedSync.destroy();
      mainSync.destroy();
    });
  });
});
