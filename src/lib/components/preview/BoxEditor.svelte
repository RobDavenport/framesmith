<script lang="ts">
  import type { FrameHitbox, Rect, State } from "$lib/types";
  import { PREVIEW_ORIGIN_Y_FRAC } from "$lib/rendercore/config";
  import {
    type Layer,
    type CornerHandle,
    type PointerAction,
    getLocalPt,
    normalizeRect,
    hitTestHandle,
    hitTestBody,
    computeMoveRect,
    computeResizeRect,
    computeDrawRect,
  } from "./useBoxDrag";

  type Tool = "select" | "draw";
  type Selection = { layer: Layer; index: number };

  type Props = {
    move: State | null;
    editLayer: Layer;
    editTool: Tool;
    frameIndex: number;
    dpr: number;
    canvasWidth: number;
    canvasHeight: number;
    message: string | null;
    selection: Selection | null;
    onSelectionChange: (sel: Selection | null) => void;
    onRectUpdate: (layer: Layer, index: number, rect: Rect) => void;
    onRectAdd: (layer: Layer, rect: Rect) => void;
  };

  let {
    move,
    editLayer,
    editTool,
    frameIndex,
    dpr,
    canvasWidth,
    canvasHeight,
    message,
    selection = $bindable(),
    onSelectionChange,
    onRectUpdate,
    onRectAdd,
  }: Props = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);

  let action = $state<PointerAction | null>(null);
  let draftRect = $state<Rect | null>(null);
  let liveOverride = $state<{ layer: Layer; index: number; rect: Rect } | null>(null);

  export function getCanvas(): HTMLCanvasElement | null {
    return canvasEl;
  }

  function getLayerArray(m: State, layer: Layer): FrameHitbox[] {
    const raw = (m as any)[layer];
    return Array.isArray(raw) ? raw : [];
  }

  function isActiveFrame(frames: [number, number] | undefined, f: number): boolean {
    if (!frames) return false;
    const start = Math.floor(frames[0] ?? 0);
    const end = Math.floor(frames[1] ?? 0);
    return start <= f && f <= end;
  }

  export function drawOverlay(ctx: CanvasRenderingContext2D): void {
    if (!move || message) return;

    const originX = Math.floor(canvasWidth * 0.5);
    const originY = Math.floor(canvasHeight * PREVIEW_ORIGIN_Y_FRAC);
    const px = dpr;

    const drawRects = (
      rects: { x: number; y: number; w: number; h: number; selected?: boolean; inactive?: boolean }[],
      stroke: string,
      fill: string
    ) => {
      if (!rects.length) return;
      ctx.lineWidth = Math.max(1, Math.round(1.5 * px));

      for (const r of rects) {
        const nr = normalizeRect({ x: r.x, y: r.y, w: r.w, h: r.h });
        const x = originX + nr.x * px;
        const y = originY + nr.y * px;
        const rw = nr.w * px;
        const rh = nr.h * px;
        if (!Number.isFinite(x + y + rw + rh)) continue;
        if (rw <= 0 || rh <= 0) continue;

        ctx.globalAlpha = r.inactive ? 0.3 : 1;
        ctx.strokeStyle = stroke;
        ctx.fillStyle = fill;
        ctx.fillRect(x, y, rw, rh);
        ctx.strokeRect(x + 0.5, y + 0.5, rw, rh);
        ctx.globalAlpha = 1;

        if (r.selected) {
          ctx.save();
          ctx.setLineDash([Math.max(2, Math.round(4 * px)), Math.max(2, Math.round(3 * px))]);
          ctx.lineWidth = Math.max(1, Math.round(2.25 * px));
          ctx.strokeStyle = "rgba(250, 204, 21, 0.95)";
          ctx.strokeRect(x + 0.5, y + 0.5, rw, rh);
          ctx.restore();
        }
      }
    };

    const selectedLayer = selection?.layer ?? null;
    const selectedIndex = selection?.index ?? -1;

    const hitboxes = getLayerArray(move, "hitboxes").map((hb, index) => ({
      ...((liveOverride && liveOverride.layer === "hitboxes" && liveOverride.index === index
        ? liveOverride.rect
        : hb.box) as Rect),
      selected: selectedLayer === "hitboxes" && selectedIndex === index,
      inactive: !isActiveFrame(hb.frames, frameIndex),
    }));

    const hurtboxes = getLayerArray(move, "hurtboxes").map((hb, index) => ({
      ...((liveOverride && liveOverride.layer === "hurtboxes" && liveOverride.index === index
        ? liveOverride.rect
        : hb.box) as Rect),
      selected: selectedLayer === "hurtboxes" && selectedIndex === index,
      inactive: !isActiveFrame(hb.frames, frameIndex),
    }));

    const pushboxes = getLayerArray(move, "pushboxes").map((hb, index) => ({
      ...((liveOverride && liveOverride.layer === "pushboxes" && liveOverride.index === index
        ? liveOverride.rect
        : hb.box) as Rect),
      selected: selectedLayer === "pushboxes" && selectedIndex === index,
      inactive: !isActiveFrame(hb.frames, frameIndex),
    }));

    // Draw pushboxes first (bottom), then hurtboxes, then hitboxes (top).
    drawRects(pushboxes, "rgba(234,179,8,0.95)", "rgba(234,179,8,0.18)");
    drawRects(hurtboxes, "rgba(34,197,94,0.95)", "rgba(34,197,94,0.18)");
    drawRects(hitboxes, "rgba(239,68,68,0.95)", "rgba(239,68,68,0.18)");

    if (action?.kind === "draw" && draftRect) {
      const nr = normalizeRect(draftRect);
      const x = originX + nr.x * px;
      const y = originY + nr.y * px;
      const rw = nr.w * px;
      const rh = nr.h * px;
      if (rw > 0 && rh > 0) {
        ctx.save();
        ctx.lineWidth = Math.max(1, Math.round(2 * px));
        ctx.setLineDash([Math.max(2, Math.round(6 * px)), Math.max(2, Math.round(4 * px))]);
        ctx.strokeStyle = "rgba(148, 163, 184, 0.95)";
        ctx.fillStyle = "rgba(148, 163, 184, 0.12)";
        ctx.fillRect(x, y, rw, rh);
        ctx.strokeRect(x + 0.5, y + 0.5, rw, rh);
        ctx.restore();
      }
    }

    if (selection && editTool === "select" && selection.layer === editLayer) {
      const arr = getLayerArray(move, selection.layer);
      const hb = arr[selection.index];
      if (hb) {
        const r = normalizeRect(hb.box);
        const corners = [
          { x: r.x, y: r.y },
          { x: r.x + r.w, y: r.y },
          { x: r.x, y: r.y + r.h },
          { x: r.x + r.w, y: r.y + r.h },
        ];

        const s = Math.max(6, Math.round(6 * px));
        ctx.fillStyle = "rgba(250, 204, 21, 0.95)";
        ctx.strokeStyle = "rgba(17, 24, 39, 0.9)";
        ctx.lineWidth = Math.max(1, Math.round(1 * px));
        for (const c of corners) {
          const cx = originX + c.x * px;
          const cy = originY + c.y * px;
          ctx.fillRect(cx - s / 2, cy - s / 2, s, s);
          ctx.strokeRect(cx - s / 2 + 0.5, cy - s / 2 + 0.5, s, s);
        }
      }
    }
  }

  function handlePointerDown(e: PointerEvent): void {
    if (!move || message || !canvasEl) return;
    if (e.button !== 0) return;

    const local = getLocalPt(e, canvasEl, dpr);
    if (!local) return;

    canvasEl.setPointerCapture(e.pointerId);

    if (editTool === "draw") {
      onSelectionChange(null);
      liveOverride = null;
      action = { kind: "draw", pointerId: e.pointerId, layer: editLayer, startPt: local };
      draftRect = { x: local.x, y: local.y, w: 0, h: 0 };
      return;
    }

    // Select tool: prefer active rects, but allow selecting inactive ones.
    const all = getLayerArray(move, editLayer).map((hb, index) => ({ hb, index }));
    const active = all.filter(({ hb }) => isActiveFrame(hb.frames, frameIndex));
    const inactive = all.filter(({ hb }) => !isActiveFrame(hb.frames, frameIndex));

    const tryPick = (list: typeof all) => {
      for (let i = list.length - 1; i >= 0; i--) {
        const { hb, index } = list[i];
        const rect = normalizeRect(hb.box);
        const handle = hitTestHandle(local, rect);
        if (handle) {
          onSelectionChange({ layer: editLayer, index });
          liveOverride = { layer: editLayer, index, rect };
          action = {
            kind: "resize",
            pointerId: e.pointerId,
            layer: editLayer,
            index,
            handle,
            startRect: rect,
          };
          return true;
        }
        if (hitTestBody(local, rect)) {
          onSelectionChange({ layer: editLayer, index });
          liveOverride = { layer: editLayer, index, rect };
          action = {
            kind: "move",
            pointerId: e.pointerId,
            layer: editLayer,
            index,
            startPt: local,
            startRect: rect,
          };
          return true;
        }
      }
      return false;
    };

    if (tryPick(active)) return;
    if (tryPick(inactive)) return;

    onSelectionChange(null);
    action = null;
  }

  function handlePointerMove(e: PointerEvent): void {
    if (!move || message) return;
    if (!action) return;
    if (e.pointerId !== action.pointerId) return;
    if (!canvasEl) return;

    const local = getLocalPt(e, canvasEl, dpr);
    if (!local) return;

    if (action.kind === "draw") {
      draftRect = computeDrawRect(action, local);
      return;
    }

    if (action.kind === "move") {
      liveOverride = {
        layer: action.layer,
        index: action.index,
        rect: computeMoveRect(action, local),
      };
      return;
    }

    // resize
    liveOverride = {
      layer: action.layer,
      index: action.index,
      rect: computeResizeRect(action, local),
    };
  }

  function handlePointerUp(e: PointerEvent): void {
    if (!action) return;
    if (e.pointerId !== action.pointerId) return;

    if (canvasEl?.hasPointerCapture(e.pointerId)) {
      canvasEl.releasePointerCapture(e.pointerId);
    }

    if (action.kind === "draw") {
      const r = draftRect;
      draftRect = null;
      const layer = action.layer;
      action = null;
      liveOverride = null;
      if (!r) return;
      const nr = normalizeRect(r);
      if (nr.w <= 0 || nr.h <= 0) return;
      onRectAdd(layer, nr);
      return;
    }

    if (action.kind === "move" || action.kind === "resize") {
      const rect =
        liveOverride && liveOverride.layer === action.layer && liveOverride.index === action.index
          ? liveOverride.rect
          : action.startRect;
      onRectUpdate(action.layer, action.index, rect);
      liveOverride = null;
      action = null;
      return;
    }

    liveOverride = null;
    action = null;
  }

  function handlePointerCancel(e: PointerEvent): void {
    if (!action) return;
    if (e.pointerId !== action.pointerId) return;

    if (canvasEl?.hasPointerCapture(e.pointerId)) {
      canvasEl.releasePointerCapture(e.pointerId);
    }
    action = null;
    draftRect = null;
    liveOverride = null;
  }
</script>

<canvas
  bind:this={canvasEl}
  class="hitbox-overlay"
  class:tool-draw={editTool === "draw"}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerCancel}
></canvas>

<style>
  .hitbox-overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    pointer-events: auto;
    touch-action: none;
    cursor: default;
    z-index: 2;
  }

  .hitbox-overlay.tool-draw {
    cursor: crosshair;
  }
</style>
