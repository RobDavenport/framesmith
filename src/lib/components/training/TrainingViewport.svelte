<script lang="ts">
  /**
   * TrainingViewport - Renders player and dummy side-by-side in training mode.
   *
   * This component provides a 2D viewport displaying both characters as
   * simple colored boxes. Real sprite/3D rendering will be added later.
   */

  interface CharacterDisplay {
    /** X position in pixels. */
    x: number;
    /** Y position in pixels. */
    y: number;
    /** Width in pixels. */
    width: number;
    /** Height in pixels. */
    height: number;
    /** Whether character is facing right. */
    facingRight: boolean;
    /** Current move name for display. */
    currentMove?: string;
  }

  interface Props {
    /** Player character display data. */
    player: CharacterDisplay;
    /** Dummy character display data. */
    dummy: CharacterDisplay;
    /** Viewport width in pixels. */
    viewportWidth?: number;
    /** Viewport height in pixels. */
    viewportHeight?: number;
    /** Ground level Y position. */
    groundY?: number;
  }

  let {
    player,
    dummy,
    viewportWidth = 800,
    viewportHeight = 400,
    groundY = 350,
  }: Props = $props();

  // Calculate camera offset to center between characters
  const cameraX = $derived((player.x + dummy.x) / 2 - viewportWidth / 2);

  // Transform world coordinates to screen coordinates
  function worldToScreen(worldX: number, worldY: number): { x: number; y: number } {
    return {
      x: worldX - cameraX,
      y: viewportHeight - worldY, // Y is inverted (world Y up, screen Y down)
    };
  }

  // Get screen position for player
  const playerScreen = $derived(worldToScreen(player.x, player.y));
  const dummyScreen = $derived(worldToScreen(dummy.x, dummy.y));
</script>

<div class="viewport" style:width="{viewportWidth}px" style:height="{viewportHeight}px">
  <!-- Ground line -->
  <div
    class="ground"
    style:bottom="{viewportHeight - groundY}px"
  ></div>

  <!-- Player character -->
  <div
    class="character player"
    class:facing-left={!player.facingRight}
    style:left="{playerScreen.x - player.width / 2}px"
    style:bottom="{player.y}px"
    style:width="{player.width}px"
    style:height="{player.height}px"
  >
    <div class="character-label">P1</div>
    {#if player.currentMove}
      <div class="move-label">{player.currentMove}</div>
    {/if}
  </div>

  <!-- Dummy character -->
  <div
    class="character dummy"
    class:facing-left={!dummy.facingRight}
    style:left="{dummyScreen.x - dummy.width / 2}px"
    style:bottom="{dummy.y}px"
    style:width="{dummy.width}px"
    style:height="{dummy.height}px"
  >
    <div class="character-label">CPU</div>
    {#if dummy.currentMove}
      <div class="move-label">{dummy.currentMove}</div>
    {/if}
  </div>

  <!-- Center marker -->
  <div class="center-marker" style:left="{viewportWidth / 2}px"></div>
</div>

<style>
  .viewport {
    position: relative;
    background: linear-gradient(
      to bottom,
      var(--bg-secondary) 0%,
      var(--bg-primary) 100%
    );
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }

  .ground {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--border);
  }

  .character {
    position: absolute;
    border-radius: 2px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    padding-top: 4px;
    transition: transform 0.05s ease-out;
  }

  .character.player {
    background: rgba(74, 222, 128, 0.6);
    border: 2px solid var(--success);
  }

  .character.dummy {
    background: rgba(233, 69, 96, 0.6);
    border: 2px solid var(--accent);
  }

  .character.facing-left {
    transform: scaleX(-1);
  }

  .character.facing-left .character-label,
  .character.facing-left .move-label {
    transform: scaleX(-1);
  }

  .character-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-primary);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .move-label {
    font-size: 9px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .center-marker {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: rgba(255, 255, 255, 0.1);
    pointer-events: none;
  }
</style>
