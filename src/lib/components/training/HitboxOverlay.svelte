<script lang="ts">
  /**
   * HitboxOverlay - Canvas overlay for visualizing hitboxes and hurtboxes.
   *
   * Draws colored rectangles representing active hitboxes (red) and hurtboxes (blue/green)
   * for both player and dummy characters.
   */

  import type { State } from '$lib/types';

  interface CharacterBoxData {
    move: State | null;
    frame: number;
    x: number;
    y: number;
    facing: 'left' | 'right';
  }

  interface Props {
    player: CharacterBoxData;
    dummy: CharacterBoxData;
    show: boolean;
    width?: number;
    height?: number;
  }

  let { player, dummy, show, width = 800, height = 400 }: Props = $props();

  let canvas: HTMLCanvasElement | null = $state(null);
  let ctx: CanvasRenderingContext2D | null = $state(null);

  // Colors
  const HITBOX_COLOR = 'rgba(255, 50, 50, 0.6)';
  const HITBOX_STROKE = 'rgba(255, 0, 0, 0.9)';
  const HURTBOX_PLAYER_COLOR = 'rgba(50, 150, 255, 0.4)';
  const HURTBOX_PLAYER_STROKE = 'rgba(50, 150, 255, 0.8)';
  const HURTBOX_DUMMY_COLOR = 'rgba(50, 255, 150, 0.4)';
  const HURTBOX_DUMMY_STROKE = 'rgba(50, 255, 150, 0.8)';
  const PUSHBOX_COLOR = 'rgba(255, 255, 0, 0.4)';
  const PUSHBOX_STROKE = '#FFFF00';
  const POSITION_MARKER_COLOR = 'rgba(255, 255, 255, 0.8)';
  const DEFAULT_BODY_COLOR = 'rgba(128, 128, 128, 0.3)';
  const DEFAULT_BODY_STROKE = 'rgba(128, 128, 128, 0.6)';

  // Ground line position (percentage from top)
  const GROUND_Y_PERCENT = 0.75;

  function isFrameActive(frames: [number, number], currentFrame: number): boolean {
    return currentFrame >= frames[0] && currentFrame <= frames[1];
  }

  function drawRect(
    ctx: CanvasRenderingContext2D,
    screenX: number,
    groundY: number,
    facing: 'left' | 'right',
    box: { x: number; y: number; w: number; h: number },
    fillColor: string,
    strokeColor: string
  ) {
    // Box coordinates are relative to character position
    // x is offset from character (positive = forward in facing direction)
    // y is offset from ground (negative = above ground in fighting game convention)
    const facingMult = facing === 'right' ? 1 : -1;

    // Calculate screen position
    // For facing right: box.x positive = to the right
    // For facing left: box.x positive = to the left (flip)
    let drawX: number;
    if (facing === 'right') {
      drawX = screenX + box.x;
    } else {
      drawX = screenX - box.x - box.w;
    }

    // Y: box.y is typically negative (above ground), so we add it to groundY
    // which moves up on screen (smaller Y value)
    const drawY = groundY + box.y;

    ctx.fillStyle = fillColor;
    ctx.fillRect(drawX, drawY, box.w, box.h);

    ctx.strokeStyle = strokeColor;
    ctx.lineWidth = 2;
    ctx.strokeRect(drawX, drawY, box.w, box.h);
  }

  function drawCircle(
    ctx: CanvasRenderingContext2D,
    screenX: number,
    groundY: number,
    facing: 'left' | 'right',
    circle: { x: number; y: number; r: number },
    fillColor: string,
    strokeColor: string
  ) {
    const facingMult = facing === 'right' ? 1 : -1;
    const cx = screenX + circle.x * facingMult;
    const cy = groundY + circle.y;

    ctx.fillStyle = fillColor;
    ctx.beginPath();
    ctx.arc(cx, cy, circle.r, 0, Math.PI * 2);
    ctx.fill();

    ctx.strokeStyle = strokeColor;
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  function drawPositionMarker(
    ctx: CanvasRenderingContext2D,
    screenX: number,
    groundY: number,
    label: string
  ) {
    ctx.strokeStyle = POSITION_MARKER_COLOR;
    ctx.lineWidth = 2;

    // Draw crosshair at feet
    const size = 10;
    ctx.beginPath();
    ctx.moveTo(screenX - size, groundY);
    ctx.lineTo(screenX + size, groundY);
    ctx.moveTo(screenX, groundY - size);
    ctx.lineTo(screenX, groundY + size);
    ctx.stroke();

    // Draw label
    ctx.fillStyle = 'white';
    ctx.font = '10px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(label, screenX, groundY + 20);
  }

  function drawDefaultBody(
    ctx: CanvasRenderingContext2D,
    screenX: number,
    groundY: number,
    facing: 'left' | 'right',
    isPlayer: boolean
  ) {
    // Draw a default body shape when no hurtbox is active
    const bodyWidth = 30;
    const bodyHeight = 60;

    const drawX = screenX - bodyWidth / 2;
    const drawY = groundY - bodyHeight;

    const color = isPlayer ? HURTBOX_PLAYER_COLOR : HURTBOX_DUMMY_COLOR;
    const stroke = isPlayer ? HURTBOX_PLAYER_STROKE : HURTBOX_DUMMY_STROKE;

    ctx.fillStyle = color;
    ctx.fillRect(drawX, drawY, bodyWidth, bodyHeight);
    ctx.strokeStyle = stroke;
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    ctx.strokeRect(drawX, drawY, bodyWidth, bodyHeight);
    ctx.setLineDash([]);
  }

  function drawCharacterBoxes(
    ctx: CanvasRenderingContext2D,
    data: CharacterBoxData,
    isPlayer: boolean,
    canvasH: number
  ) {
    const { move, frame, x, facing } = data;
    const groundY = canvasH * GROUND_Y_PERCENT;

    // Draw position marker
    drawPositionMarker(ctx, x, groundY, isPlayer ? 'P1' : 'CPU');

    const hurtboxColor = isPlayer ? HURTBOX_PLAYER_COLOR : HURTBOX_DUMMY_COLOR;
    const hurtboxStroke = isPlayer ? HURTBOX_PLAYER_STROKE : HURTBOX_DUMMY_STROKE;

    let hasActiveHurtbox = false;

    if (move) {
      // Draw hurtboxes (legacy format)
      for (const hurtbox of move.hurtboxes) {
        if (isFrameActive(hurtbox.frames, frame)) {
          drawRect(ctx, x, groundY, facing, hurtbox.box, hurtboxColor, hurtboxStroke);
          hasActiveHurtbox = true;
        }
      }

      // Draw advanced hurtboxes if present
      if (move.advanced_hurtboxes) {
        for (const frameHurtbox of move.advanced_hurtboxes) {
          if (isFrameActive(frameHurtbox.frames, frame)) {
            for (const shape of frameHurtbox.boxes) {
              if (shape.type === 'aabb' || shape.type === 'rect') {
                const box = { x: shape.x, y: shape.y, w: shape.w, h: shape.h };
                drawRect(ctx, x, groundY, facing, box, hurtboxColor, hurtboxStroke);
                hasActiveHurtbox = true;
              } else if (shape.type === 'circle') {
                drawCircle(ctx, x, groundY, facing, shape, hurtboxColor, hurtboxStroke);
                hasActiveHurtbox = true;
              }
            }
          }
        }
      }

      // Draw pushboxes
      if (move.pushboxes) {
        for (const pb of move.pushboxes) {
          if (isFrameActive(pb.frames, frame)) {
            drawRect(ctx, x, groundY, facing, pb.box, PUSHBOX_COLOR, PUSHBOX_STROKE);
          }
        }
      }

      // Draw hitboxes (legacy format)
      for (const hitbox of move.hitboxes) {
        if (isFrameActive(hitbox.frames, frame)) {
          drawRect(ctx, x, groundY, facing, hitbox.box, HITBOX_COLOR, HITBOX_STROKE);
        }
      }

      // Draw v2 hits if present
      if (move.hits) {
        for (const hit of move.hits) {
          if (isFrameActive(hit.frames, frame)) {
            for (const shape of hit.hitboxes) {
              if (shape.type === 'aabb' || shape.type === 'rect') {
                const box = { x: shape.x, y: shape.y, w: shape.w, h: shape.h };
                drawRect(ctx, x, groundY, facing, box, HITBOX_COLOR, HITBOX_STROKE);
              } else if (shape.type === 'circle') {
                drawCircle(ctx, x, groundY, facing, shape, HITBOX_COLOR, HITBOX_STROKE);
              }
            }
          }
        }
      }
    }

    // Draw default body outline if no hurtbox is active
    if (!hasActiveHurtbox) {
      drawDefaultBody(ctx, x, groundY, facing, isPlayer);
    }
  }

  function drawDebugInfo(ctx: CanvasRenderingContext2D, data: CharacterBoxData, label: string, yOffset: number) {
    ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
    ctx.fillRect(5, yOffset, 180, 50);

    ctx.fillStyle = 'white';
    ctx.font = '11px monospace';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';

    const moveName = data.move?.input ?? 'none';
    const totalFrames = data.move ? data.move.startup + data.move.active + data.move.recovery : 0;

    ctx.fillText(`${label}: ${moveName}`, 10, yOffset + 5);
    ctx.fillText(`Frame: ${data.frame} / ${totalFrames}`, 10, yOffset + 20);
    ctx.fillText(`Pos: (${data.x}, ${data.y})`, 10, yOffset + 35);
  }

  function render() {
    if (!ctx || !canvas) return;
    if (!show) {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      return;
    }

    const canvasW = canvas.width;
    const canvasH = canvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, canvasW, canvasH);

    // Draw ground line
    const groundY = canvasH * GROUND_Y_PERCENT;
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.3)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, groundY);
    ctx.lineTo(canvasW, groundY);
    ctx.stroke();

    // Draw center line
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(canvasW / 2, 0);
    ctx.lineTo(canvasW / 2, canvasH);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw character boxes
    drawCharacterBoxes(ctx, player, true, canvasH);
    drawCharacterBoxes(ctx, dummy, false, canvasH);

    // Draw debug info
    drawDebugInfo(ctx, player, 'P1', 5);
    drawDebugInfo(ctx, dummy, 'CPU', 60);

    // Draw legend
    drawLegend(ctx, canvasW);
  }

  function drawLegend(ctx: CanvasRenderingContext2D, canvasW: number) {
    const legendX = canvasW - 120;
    const legendY = 10;
    const boxSize = 12;
    const spacing = 18;

    ctx.font = '11px monospace';
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'left';

    // Hitbox
    ctx.fillStyle = HITBOX_COLOR;
    ctx.fillRect(legendX, legendY, boxSize, boxSize);
    ctx.strokeStyle = HITBOX_STROKE;
    ctx.lineWidth = 1;
    ctx.strokeRect(legendX, legendY, boxSize, boxSize);
    ctx.fillStyle = 'white';
    ctx.fillText('Hitbox', legendX + boxSize + 6, legendY + boxSize / 2);

    // Player hurtbox
    ctx.fillStyle = HURTBOX_PLAYER_COLOR;
    ctx.fillRect(legendX, legendY + spacing, boxSize, boxSize);
    ctx.strokeStyle = HURTBOX_PLAYER_STROKE;
    ctx.strokeRect(legendX, legendY + spacing, boxSize, boxSize);
    ctx.fillStyle = 'white';
    ctx.fillText('P1 Hurtbox', legendX + boxSize + 6, legendY + spacing + boxSize / 2);

    // Dummy hurtbox
    ctx.fillStyle = HURTBOX_DUMMY_COLOR;
    ctx.fillRect(legendX, legendY + spacing * 2, boxSize, boxSize);
    ctx.strokeStyle = HURTBOX_DUMMY_STROKE;
    ctx.strokeRect(legendX, legendY + spacing * 2, boxSize, boxSize);
    ctx.fillStyle = 'white';
    ctx.fillText('CPU Hurtbox', legendX + boxSize + 6, legendY + spacing * 2 + boxSize / 2);

    // Pushbox
    ctx.fillStyle = PUSHBOX_COLOR;
    ctx.fillRect(legendX, legendY + spacing * 3, boxSize, boxSize);
    ctx.strokeStyle = PUSHBOX_STROKE;
    ctx.strokeRect(legendX, legendY + spacing * 3, boxSize, boxSize);
    ctx.fillStyle = 'white';
    ctx.fillText('Pushbox', legendX + boxSize + 6, legendY + spacing * 3 + boxSize / 2);
  }

  $effect(() => {
    if (canvas) {
      ctx = canvas.getContext('2d');
      // Set canvas resolution
      canvas.width = width;
      canvas.height = height;
    }
  });

  $effect(() => {
    // Re-render when any dependency changes
    void [player, dummy, show, width, height];
    render();
  });
</script>

<canvas
  bind:this={canvas}
  class="hitbox-overlay"
  class:hidden={!show}
  style="width: {width}px; height: {height}px;"
></canvas>

<style>
  .hitbox-overlay {
    position: absolute;
    top: 0;
    left: 0;
    pointer-events: none;
    z-index: 10;
  }

  .hitbox-overlay.hidden {
    display: none;
  }
</style>
