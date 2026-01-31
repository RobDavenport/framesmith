/**
 * TrainingSync - BroadcastChannel-based sync between main and detached training windows.
 *
 * This module provides:
 * - Cross-window character data synchronization
 * - Live sync (instant updates) vs sync-on-save modes
 * - Message typing for type-safe communication
 */

import type { CharacterData } from '$lib/types';

/** Sync mode preference for training mode updates. */
export type SyncMode = 'live' | 'on-save';

/** Message types for the training sync channel. */
export type TrainingSyncMessage =
  | { type: 'character-change'; character: CharacterData }
  | { type: 'character-save'; character: CharacterData }
  | { type: 'project-path'; path: string | null }
  | { type: 'request-sync' }
  | { type: 'ping' }
  | { type: 'pong' };

/** Channel name for training mode sync. */
const CHANNEL_NAME = 'framesmith-training-sync';

/** Callbacks for handling sync messages. */
export interface TrainingSyncCallbacks {
  /** Called when character data changes (live sync). */
  onCharacterChange?: (character: CharacterData) => void;
  /** Called when character is saved (sync-on-save). */
  onCharacterSave?: (character: CharacterData) => void;
  /** Called when project path changes. */
  onProjectPath?: (path: string | null) => void;
  /** Called when another window requests sync. */
  onSyncRequest?: () => void;
}

/**
 * TrainingSync - Manages BroadcastChannel communication for training mode.
 *
 * Main window usage:
 * ```typescript
 * const sync = new TrainingSync();
 * // Send updates when character changes
 * sync.sendCharacterChange(characterData);
 * // Send updates when character is saved
 * sync.sendCharacterSave(characterData);
 * ```
 *
 * Detached window usage:
 * ```typescript
 * const sync = new TrainingSync({
 *   onCharacterChange: (data) => reloadCharacter(data),
 *   onCharacterSave: (data) => reloadCharacter(data),
 * });
 * // Request initial data
 * sync.requestSync();
 * ```
 */
export class TrainingSync {
  private channel: BroadcastChannel | null = null;
  private callbacks: TrainingSyncCallbacks;
  private isDestroyed = false;

  constructor(callbacks: TrainingSyncCallbacks = {}) {
    this.callbacks = callbacks;

    // BroadcastChannel is not available in all environments (e.g., some test runners)
    if (typeof BroadcastChannel !== 'undefined') {
      this.channel = new BroadcastChannel(CHANNEL_NAME);
      this.channel.onmessage = this.handleMessage.bind(this);
    }
  }

  /** Handle incoming messages from the channel. */
  private handleMessage(event: MessageEvent<TrainingSyncMessage>): void {
    if (this.isDestroyed) return;

    const message = event.data;

    switch (message.type) {
      case 'character-change':
        this.callbacks.onCharacterChange?.(message.character);
        break;
      case 'character-save':
        this.callbacks.onCharacterSave?.(message.character);
        break;
      case 'project-path':
        this.callbacks.onProjectPath?.(message.path);
        break;
      case 'request-sync':
        this.callbacks.onSyncRequest?.();
        break;
      case 'ping':
        this.sendMessage({ type: 'pong' });
        break;
      case 'pong':
        // Acknowledgment received, no action needed
        break;
    }
  }

  /** Send a message to all other windows on the channel. */
  private sendMessage(message: TrainingSyncMessage): void {
    if (this.isDestroyed || !this.channel) return;
    this.channel.postMessage(message);
  }

  /** Send character change event (for live sync mode). */
  sendCharacterChange(character: CharacterData): void {
    this.sendMessage({ type: 'character-change', character });
  }

  /** Send character save event (for sync-on-save mode). */
  sendCharacterSave(character: CharacterData): void {
    this.sendMessage({ type: 'character-save', character });
  }

  /** Send project path update. */
  sendProjectPath(path: string | null): void {
    this.sendMessage({ type: 'project-path', path });
  }

  /** Request sync from other windows (used by detached window on startup). */
  requestSync(): void {
    this.sendMessage({ type: 'request-sync' });
  }

  /** Check if another training window is open by sending a ping. */
  ping(): void {
    this.sendMessage({ type: 'ping' });
  }

  /** Update callbacks after construction. */
  setCallbacks(callbacks: TrainingSyncCallbacks): void {
    this.callbacks = callbacks;
  }

  /** Clean up the channel. */
  destroy(): void {
    this.isDestroyed = true;
    if (this.channel) {
      this.channel.close();
      this.channel = null;
    }
  }
}

/**
 * Create a training sync instance configured for the main editor window.
 *
 * The main window sends updates and responds to sync requests.
 */
export function createMainWindowSync(
  getCharacter: () => CharacterData | null,
  getProjectPath: () => string | null
): TrainingSync {
  const sync = new TrainingSync({
    onSyncRequest: () => {
      // Send current state to requester
      const character = getCharacter();
      if (character) {
        sync.sendCharacterChange(character);
      }
      sync.sendProjectPath(getProjectPath());
    },
  });

  return sync;
}

/**
 * Create a training sync instance configured for a detached training window.
 *
 * The detached window receives updates and can request sync.
 */
export function createDetachedWindowSync(
  onCharacterUpdate: (character: CharacterData) => void,
  onProjectPath: (path: string | null) => void,
  syncMode: SyncMode = 'live'
): TrainingSync {
  const handleUpdate = (character: CharacterData) => {
    onCharacterUpdate(character);
  };

  const callbacks: TrainingSyncCallbacks =
    syncMode === 'live'
      ? {
          onCharacterChange: handleUpdate,
          onCharacterSave: handleUpdate,
          onProjectPath,
        }
      : {
          // In sync-on-save mode, only respond to save events
          onCharacterSave: handleUpdate,
          onProjectPath,
        };

  return new TrainingSync(callbacks);
}
