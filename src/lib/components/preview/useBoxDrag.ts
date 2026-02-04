import type { Rect } from "$lib/types";
import { PREVIEW_ORIGIN_Y_FRAC } from "$lib/rendercore/config";

export type CornerHandle = "nw" | "ne" | "sw" | "se";
export type Layer = "hitboxes" | "hurtboxes" | "pushboxes";

export type PointerAction =
  | {
      kind: "move";
      pointerId: number;
      layer: Layer;
      index: number;
      startPt: { x: number; y: number };
      startRect: Rect;
    }
  | {
      kind: "resize";
      pointerId: number;
      layer: Layer;
      index: number;
      handle: CornerHandle;
      startRect: Rect;
    }
  | {
      kind: "draw";
      pointerId: number;
      layer: Layer;
      startPt: { x: number; y: number };
    };

/**
 * Converts a screen-space pointer event to local world coordinates.
 * Origin is at center horizontally and PREVIEW_ORIGIN_Y_FRAC vertically.
 */
export function getLocalPt(
  e: PointerEvent,
  canvasEl: HTMLCanvasElement,
  dpr: number
): { x: number; y: number } | null {
  const rect = canvasEl.getBoundingClientRect();
  const deviceX = (e.clientX - rect.left) * dpr;
  const deviceY = (e.clientY - rect.top) * dpr;

  const originX = Math.floor(canvasEl.width * 0.5);
  const originY = Math.floor(canvasEl.height * PREVIEW_ORIGIN_Y_FRAC);

  return {
    x: (deviceX - originX) / dpr,
    y: (deviceY - originY) / dpr,
  };
}

/**
 * Normalizes a rect so width/height are positive.
 */
export function normalizeRect(r: Rect): Rect {
  let { x, y, w, h } = r;
  if (!Number.isFinite(x)) x = 0;
  if (!Number.isFinite(y)) y = 0;
  if (!Number.isFinite(w)) w = 0;
  if (!Number.isFinite(h)) h = 0;
  if (w < 0) {
    x += w;
    w = -w;
  }
  if (h < 0) {
    y += h;
    h = -h;
  }
  return { x, y, w, h };
}

/**
 * Snaps rect coordinates to integers and ensures minimum size of 1x1.
 */
export function snapRect(r: Rect): Rect {
  const nr = normalizeRect(r);
  const x = Math.round(nr.x);
  const y = Math.round(nr.y);
  const w = Math.max(1, Math.round(nr.w));
  const h = Math.max(1, Math.round(nr.h));
  return { x, y, w, h };
}

/**
 * Hit-tests corner handles (6px radius).
 * Returns the handle identifier if hit, null otherwise.
 */
export function hitTestHandle(local: { x: number; y: number }, rect: Rect): CornerHandle | null {
  const r = normalizeRect(rect);
  const handleRadius = 6;
  const corners: { h: CornerHandle; x: number; y: number }[] = [
    { h: "nw", x: r.x, y: r.y },
    { h: "ne", x: r.x + r.w, y: r.y },
    { h: "sw", x: r.x, y: r.y + r.h },
    { h: "se", x: r.x + r.w, y: r.y + r.h },
  ];
  for (const c of corners) {
    const dx = local.x - c.x;
    const dy = local.y - c.y;
    if (Math.abs(dx) <= handleRadius && Math.abs(dy) <= handleRadius) return c.h;
  }
  return null;
}

/**
 * Hit-tests the body (interior) of a rect.
 */
export function hitTestBody(local: { x: number; y: number }, rect: Rect): boolean {
  const r = normalizeRect(rect);
  return r.x <= local.x && local.x <= r.x + r.w && r.y <= local.y && local.y <= r.y + r.h;
}

/**
 * Computes the updated rect during a move action.
 */
export function computeMoveRect(action: Extract<PointerAction, { kind: "move" }>, local: { x: number; y: number }): Rect {
  const dx = local.x - action.startPt.x;
  const dy = local.y - action.startPt.y;
  return {
    x: action.startRect.x + dx,
    y: action.startRect.y + dy,
    w: action.startRect.w,
    h: action.startRect.h,
  };
}

/**
 * Computes the updated rect during a resize action.
 */
export function computeResizeRect(
  action: Extract<PointerAction, { kind: "resize" }>,
  local: { x: number; y: number }
): Rect {
  const r = action.startRect;
  const x1 = r.x;
  const y1 = r.y;
  const x2 = r.x + r.w;
  const y2 = r.y + r.h;

  const fixed =
    action.handle === "nw"
      ? { x: x2, y: y2 }
      : action.handle === "ne"
      ? { x: x1, y: y2 }
      : action.handle === "sw"
      ? { x: x2, y: y1 }
      : { x: x1, y: y1 };

  return {
    x: Math.min(local.x, fixed.x),
    y: Math.min(local.y, fixed.y),
    w: Math.abs(local.x - fixed.x),
    h: Math.abs(local.y - fixed.y),
  };
}

/**
 * Computes the draft rect during a draw action.
 */
export function computeDrawRect(action: Extract<PointerAction, { kind: "draw" }>, local: { x: number; y: number }): Rect {
  return {
    x: action.startPt.x,
    y: action.startPt.y,
    w: local.x - action.startPt.x,
    h: local.y - action.startPt.y,
  };
}
