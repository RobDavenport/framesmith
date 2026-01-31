<script lang="ts">
  /**
   * TrainingViewport - Renders player and dummy side-by-side in training mode.
   *
   * This component provides a 2D viewport displaying both characters as
   * simple colored boxes. Real sprite/3D rendering will be added later.
   *
   * Supports hitbox overlay visualization:
   * - Red = hitboxes (attack areas)
   * - Green = hurtboxes (vulnerable areas)
   * - Blue = pushboxes (collision)
   */

  /**
   * A collision box for overlay rendering.
   */
  export interface CollisionBox {
    /** X offset from character center. */
    x: number;
    /** Y offset from character position. */
    y: number;
    /** Width of the box. */
    width: number;
    /** Height of the box. */
    height: number;
  }

  /**
   * Hitbox data for a character.
   */
  export interface HitboxData {
    /** Attack hitboxes (red). */
    hitboxes: CollisionBox[];
    /** Vulnerable hurtboxes (green). */
    hurtboxes: CollisionBox[];
    /** Collision pushbox (blue). */
    pushbox?: CollisionBox;
  }

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
    /** Whether to show hitbox overlay. */
    showHitboxes?: boolean;
    /** Player hitbox data. */
    playerHitboxes?: HitboxData;
    /** Dummy hitbox data. */
    dummyHitboxes?: HitboxData;
  }

  let {
    player,
    dummy,
    viewportWidth = 800,
    viewportHeight = 400,
    groundY = 350,
    showHitboxes = false,
    playerHitboxes,
    dummyHitboxes,
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

  /**
   * Calculate screen position for a collision box.
   */
  function getBoxScreenPosition(
    charX: number,
    charY: number,
    box: CollisionBox,
    facingRight: boolean
  ): { left: number; bottom: number; width: number; height: number } {
    // Flip X offset based on facing direction
    const xOffset = facingRight ? box.x : -box.x - box.width;
    const screenX = charX - cameraX + xOffset;

    return {
      left: screenX,
      bottom: charY + box.y,
      width: box.width,
      height: box.height,
    };
  }
</script>

<div class="viewport" style:width="{viewportWidth}px" style:height="{viewportHeight}px">
  <!-- Ground line -->
  <div
    class="ground"
    style:bottom="{viewportHeight - groundY}px"
  ></div>

  <!-- Player character -->
  <!--
    Note on coordinate system:
    - X uses screen coordinates via playerScreen.x (camera-adjusted)
    - Y uses CSS `bottom` positioning where 0 = ground level, not screen coordinates.
      This is intentional: higher Y values = character higher off the ground.
  -->
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

  <!-- Hitbox overlays -->
  {#if showHitboxes}
    <!-- Player hitboxes -->
    {#if playerHitboxes}
      <!-- Pushbox (blue) -->
      {#if playerHitboxes.pushbox}
        {@const pos = getBoxScreenPosition(player.x, player.y, playerHitboxes.pushbox, player.facingRight)}
        <div
          class="hitbox pushbox"
          style:left="{pos.left}px"
          style:bottom="{pos.bottom}px"
          style:width="{pos.width}px"
          style:height="{pos.height}px"
        ></div>
      {/if}
      <!-- Hurtboxes (green) -->
      {#each playerHitboxes.hurtboxes as hurtbox}
        {@const pos = getBoxScreenPosition(player.x, player.y, hurtbox, player.facingRight)}
        <div
          class="hitbox hurtbox"
          style:left="{pos.left}px"
          style:bottom="{pos.bottom}px"
          style:width="{pos.width}px"
          style:height="{pos.height}px"
        ></div>
      {/each}
      <!-- Hitboxes (red) -->
      {#each playerHitboxes.hitboxes as hitbox}
        {@const pos = getBoxScreenPosition(player.x, player.y, hitbox, player.facingRight)}
        <div
          class="hitbox attack"
          style:left="{pos.left}px"
          style:bottom="{pos.bottom}px"
          style:width="{pos.width}px"
          style:height="{pos.height}px"
        ></div>
      {/each}
    {/if}

    <!-- Dummy hitboxes -->
    {#if dummyHitboxes}
      <!-- Pushbox (blue) -->
      {#if dummyHitboxes.pushbox}
        {@const pos = getBoxScreenPosition(dummy.x, dummy.y, dummyHitboxes.pushbox, dummy.facingRight)}
        <div
          class="hitbox pushbox"
          style:left="{pos.left}px"
          style:bottom="{pos.bottom}px"
          style:width="{pos.width}px"
          style:height="{pos.height}px"
        ></div>
      {/if}
      <!-- Hurtboxes (green) -->
      {#each dummyHitboxes.hurtboxes as hurtbox}
        {@const pos = getBoxScreenPosition(dummy.x, dummy.y, hurtbox, dummy.facingRight)}
        <div
          class="hitbox hurtbox"
          style:left="{pos.left}px"
          style:bottom="{pos.bottom}px"
          style:width="{pos.width}px"
          style:height="{pos.height}px"
        ></div>
      {/each}
      <!-- Hitboxes (red) -->
      {#each dummyHitboxes.hitboxes as hitbox}
        {@const pos = getBoxScreenPosition(dummy.x, dummy.y, hitbox, dummy.facingRight)}
        <div
          class="hitbox attack"
          style:left="{pos.left}px"
          style:bottom="{pos.bottom}px"
          style:width="{pos.width}px"
          style:height="{pos.height}px"
        ></div>
      {/each}
    {/if}
  {/if}

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

  /* Hitbox overlay styles */
  .hitbox {
    position: absolute;
    pointer-events: none;
    border-width: 2px;
    border-style: solid;
  }

  .hitbox.attack {
    /* Red - attack hitbox */
    background: rgba(239, 68, 68, 0.3);
    border-color: rgba(239, 68, 68, 0.8);
  }

  .hitbox.hurtbox {
    /* Green - vulnerable hurtbox */
    background: rgba(34, 197, 94, 0.3);
    border-color: rgba(34, 197, 94, 0.8);
  }

  .hitbox.pushbox {
    /* Blue - collision pushbox */
    background: rgba(59, 130, 246, 0.2);
    border-color: rgba(59, 130, 246, 0.6);
  }
</style>
