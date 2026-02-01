<script lang="ts">
  import type { AnimationClip, CharacterAssets, FrameHitbox, State, Rect } from "$lib/types";
  import { onDestroy } from "svelte";

  import { getProjectPath } from "$lib/stores/project.svelte";
  import { RenderCore } from "$lib/rendercore/RenderCore";
  import { TauriAssetProvider } from "$lib/rendercore/assets/TauriAssetProvider";
  import { buildActorSpec } from "$lib/rendercore/buildActorSpec";
  import { PREVIEW_ORIGIN_Y_FRAC } from "$lib/rendercore/config";
  import type { ActorSpec } from "$lib/rendercore/types";

  type Layer = "hitboxes" | "hurtboxes";
  type Tool = "select" | "draw";
  type Selection = { layer: Layer; index: number };
  type CornerHandle = "nw" | "ne" | "sw" | "se";

  type PointerAction =
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

  type Props = {
    characterId: string | null;
    selectionKey?: string | null;
    move: State | null;
    assets: CharacterAssets | null;
    assetsLoading?: boolean;
    assetsError?: string | null;
    onMoveChange?: (next: State) => void;
  };

  const speedOptions = [0.25, 0.5, 1, 2] as const;

  let {
    characterId,
    selectionKey = null,
    move,
    assets,
    assetsLoading = false,
    assetsError = null,
    onMoveChange = undefined,
  }: Props = $props();

  let frameIndex = $state(0);
  let playing = $state(false);
  let speed = $state<(typeof speedOptions)[number]>(1);

  let viewportEl = $state<HTMLDivElement | null>(null);
  let overlayCanvasEl = $state<HTMLCanvasElement | null>(null);

  let core = $state<RenderCore | null>(null);
  let coreKey = $state<string | null>(null);
  let coreActorError = $state<string | null>(null);

  const ACTOR_ID = "p1";

  let editLayer = $state<Layer>("hitboxes");
  let editTool = $state<Tool>("select");
  let selection = $state<Selection | null>(null);
  let action = $state<PointerAction | null>(null);
  let draftRect = $state<Rect | null>(null);
  let liveOverride = $state<{ layer: Layer; index: number; rect: Rect } | null>(null);

  let resizeObs: ResizeObserver | null = null;
  let resizeRaf = 0;
  let lastDpr = 1;

  const effectiveTotal = $derived.by(() => {
    if (!move) return 1;
    const fallback = move.startup + move.active + move.recovery;
    const raw = move.total ?? fallback;
    const n = Number.isFinite(raw) ? Math.floor(raw) : 0;
    return Math.max(1, n);
  });

  const animationKey = $derived((move?.animation ?? "").trim());
  const clip = $derived.by((): AnimationClip | null => {
    if (!assets) return null;
    if (!animationKey) return null;
    return assets.animations[animationKey] ?? null;
  });

  const spriteClip = $derived.by((): Extract<AnimationClip, { mode: "sprite" }> | null => {
    if (!clip || clip.mode !== "sprite") return null;
    return clip;
  });

  const gltfClip = $derived.by((): Extract<AnimationClip, { mode: "gltf" }> | null => {
    if (!clip || clip.mode !== "gltf") return null;
    return clip;
  });

  const spriteTexturePath = $derived.by((): string | null => {
    if (!assets || !spriteClip) return null;
    return assets.textures[spriteClip.texture] ?? null;
  });

  const gltfModelPath = $derived.by((): string | null => {
    if (!assets || !gltfClip) return null;
    return assets.models[gltfClip.model] ?? null;
  });

  const charactersDir = $derived.by((): string | null => {
    const projectPath = getProjectPath();
    if (!projectPath) return null;
    return `${projectPath}/characters`;
  });

  const actorBuild = $derived.by(() => {
    if (!clip) return { spec: null as ActorSpec | null, error: null as string | null };
    if (clip.mode === "sprite") {
      return buildActorSpec({
        id: ACTOR_ID,
        pos: { x: 0, y: 0 },
        facing: "right",
        clip,
        texturePath: spriteTexturePath,
        frameIndex,
      });
    }

    return buildActorSpec({
      id: ACTOR_ID,
      pos: { x: 0, y: 0 },
      facing: "right",
      clip,
      modelPath: gltfModelPath,
      frameIndex,
    });
  });

  const actorBuildError = $derived.by((): string | null => {
    if (!clip) return null;
    if (clip.mode === "sprite") {
      return buildActorSpec({
        id: ACTOR_ID,
        pos: { x: 0, y: 0 },
        facing: "right",
        clip,
        texturePath: spriteTexturePath,
        frameIndex: 0,
      }).error;
    }
    return buildActorSpec({
      id: ACTOR_ID,
      pos: { x: 0, y: 0 },
      facing: "right",
      clip,
      modelPath: gltfModelPath,
      frameIndex: 0,
    }).error;
  });

  function clampFrame(next: number): number {
    const max = effectiveTotal - 1;
    return Math.max(0, Math.min(max, next));
  }

  function step(delta: number): void {
    playing = false;
    frameIndex = clampFrame(frameIndex + delta);
  }

  function togglePlay(): void {
    playing = !playing;
  }

  let rafId = 0;
  let lastTs = 0;
  let accumulator = 0;

  $effect(() => {
    // Reset playback when switching moves.
    const moveKey = selectionKey ?? move?.input ?? "";
    frameIndex = 0;
    playing = false;
    selection = null;
    action = null;
    draftRect = null;
    liveOverride = null;
    void moveKey;
  });

  $effect(() => {
    // Clamp when total changes.
    frameIndex = clampFrame(frameIndex);
  });

  function stopRaf(): void {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = 0;
    lastTs = 0;
    accumulator = 0;
  }

  $effect(() => {
    stopRaf();
    if (!playing) return;

    const baseFps = 60;
    const secondsPerFrame = 1 / baseFps / speed;

    const tick = (ts: number) => {
      if (!playing) return;
      if (!lastTs) lastTs = ts;

      const dt = (ts - lastTs) / 1000;
      lastTs = ts;
      accumulator += dt;

      while (accumulator >= secondsPerFrame) {
        accumulator -= secondsPerFrame;
        if (frameIndex >= effectiveTotal - 1) {
          frameIndex = effectiveTotal - 1;
          playing = false;
          stopRaf();
          return;
        }
        frameIndex = frameIndex + 1;
      }

      rafId = requestAnimationFrame(tick);
    };

    rafId = requestAnimationFrame(tick);
    return () => stopRaf();
  });

  const message = $derived.by(() => {
    if (!characterId) return "No character selected";
    if (!charactersDir) return "No project open";
    if (assetsError) return `Assets error: ${assetsError}`;
    if (assetsLoading && !assets) return "Loading assets...";
    if (!assets) return "No assets loaded (assets.json not loaded yet)";
    if (!animationKey) return "Move.animation is empty";
    if (!clip) return `Animation key not found: '${animationKey}'`;

    if (clip.mode === "sprite") {
      if (!assets.textures[clip.texture]) return `Texture key not found: '${clip.texture}'`;
    }

    if (clip.mode === "gltf") {
      if (!assets.models[clip.model]) return `Model key not found: '${clip.model}'`;
    }

    if (actorBuildError) return `Animation preview error: ${actorBuildError}`;

    return null;
  });

  function ensureCoreViewportSize(): void {
    if (!core || !viewportEl) return;
    if (typeof window === "undefined") return;
    const rect = viewportEl.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return;
    const dpr = Math.max(1, window.devicePixelRatio || 1);
    core.setViewportSize(rect.width, rect.height, dpr);
  }

  $effect(() => {
    const host = viewportEl;
    const dir = charactersDir;

    if (!host || !dir || !characterId || message) {
      coreActorError = null;
      if (core) {
        core.unmount();
        core = null;
        coreKey = null;
      }
      return;
    }

    const key = `${dir}::${characterId}`;
    if (!core || coreKey !== key) {
      core?.unmount();

      const next = new RenderCore(() => new TauriAssetProvider(dir, characterId));
      next.setClockMode("manual");
      next.setSceneMode("single");
      next.mount(host);

      core = next;
      coreKey = key;
    }

    ensureCoreViewportSize();
  });

  $effect(() => {
    if (!core) return;
    if (message) return;
    const spec = actorBuild.spec;
    if (!spec) return;

    core.setActors([spec]);
    core.renderOnce();

    const status = core.getActorStatus(spec.id);
    const initError = core.getInitError();
    coreActorError = initError ? `RenderCore init error: ${initError}` : (status.error ?? null);
  });

  $effect(() => {
    const c = core;
    if (!c) return;
    if (message) return;

    const refresh = () => {
      const initError = c.getInitError();
      if (initError) {
        coreActorError = `RenderCore init error: ${initError}`;
        return;
      }

      const status = c.getActorStatus(ACTOR_ID);
      coreActorError = status.error ?? null;
    };

    refresh();
    const handle = window.setInterval(refresh, 250);
    return () => window.clearInterval(handle);
  });

  const phases = $derived.by(() => {
    const startup = Math.max(0, Math.floor(move?.startup ?? 0));
    const active = Math.max(0, Math.floor(move?.active ?? 0));
    const recovery = Math.max(0, Math.floor(move?.recovery ?? 0));
    const sum = startup + active + recovery;
    const total = effectiveTotal;
    const extra = Math.max(0, total - sum);
    return { startup, active, recovery, extra, total };
  });

  const frameMarkerPct = $derived.by(() => {
    const denom = Math.max(1, effectiveTotal - 1);
    return (clampFrame(frameIndex) / denom) * 100;
  });

  function isActiveFrame(frames: [number, number] | undefined, f: number): boolean {
    if (!frames) return false;
    const start = Math.floor(frames[0] ?? 0);
    const end = Math.floor(frames[1] ?? 0);
    return start <= f && f <= end;
  }

  function clampFrameIndex(n: number): number {
    const max = effectiveTotal - 1;
    const v = Number.isFinite(n) ? Math.floor(n) : 0;
    return Math.max(0, Math.min(max, v));
  }

  function normalizeRect(r: Rect): Rect {
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

  function snapRect(r: Rect): Rect {
    const nr = normalizeRect(r);
    const x = Math.round(nr.x);
    const y = Math.round(nr.y);
    const w = Math.max(1, Math.round(nr.w));
    const h = Math.max(1, Math.round(nr.h));
    return { x, y, w, h };
  }

  function getLayerArray(m: State, layer: Layer): FrameHitbox[] {
    const raw = (m as any)[layer];
    return Array.isArray(raw) ? raw : [];
  }

  function applyLayerChange(layer: Layer, updater: (prev: FrameHitbox[]) => FrameHitbox[]): void {
    if (!move) return;
    if (!onMoveChange) return;
    const prevArr = getLayerArray(move, layer);
    const nextArr = updater(prevArr);
    const next: any = { ...move };
    next[layer] = nextArr;
    onMoveChange(next as State);
  }

  function ensureSelectionValid(): void {
    if (!move) {
      selection = null;
      return;
    }
    if (!selection) return;
    const arr = getLayerArray(move, selection.layer);
    if (selection.index < 0 || selection.index >= arr.length) selection = null;
  }

  function updateRect(layer: Layer, index: number, nextRect: Rect): void {
    const snapped = snapRect(nextRect);
    applyLayerChange(layer, (arr) => {
      if (index < 0 || index >= arr.length) return arr;
      const nextArr = arr.slice();
      nextArr[index] = { ...nextArr[index], box: snapped };
      return nextArr;
    });
  }

  function updateFrames(layer: Layer, index: number, nextFrames: [number, number]): void {
    const a = clampFrameIndex(nextFrames[0]);
    const b = clampFrameIndex(nextFrames[1]);
    const start = Math.min(a, b);
    const end = Math.max(a, b);
    applyLayerChange(layer, (arr) => {
      if (index < 0 || index >= arr.length) return arr;
      const nextArr = arr.slice();
      nextArr[index] = { ...nextArr[index], frames: [start, end] };
      return nextArr;
    });
  }

  function deleteSelected(): void {
    if (!move) return;
    if (!selection) return;
    const { layer, index } = selection;
    applyLayerChange(layer, (arr) => {
      if (index < 0 || index >= arr.length) return arr;
      return arr.filter((_, i) => i !== index);
    });
    selection = null;
  }

  function addRect(layer: Layer, rect: Rect): void {
    const snapped = snapRect(rect);
    const f = clampFrameIndex(frameIndex);
    const frames: [number, number] = [f, f];

    let nextIndex = 0;
    applyLayerChange(layer, (arr) => {
      nextIndex = arr.length;
      return [...arr, { frames, box: snapped }];
    });
    selection = { layer, index: nextIndex };
  }

  function getLocalPt(e: PointerEvent): { x: number; y: number } | null {
    if (!overlayCanvasEl) return null;
    ensureOverlaySize();
    const rect = overlayCanvasEl.getBoundingClientRect();
    const px = lastDpr;
    const deviceX = (e.clientX - rect.left) * px;
    const deviceY = (e.clientY - rect.top) * px;

    const originX = Math.floor(overlayCanvasEl.width * 0.5);
    const originY = Math.floor(overlayCanvasEl.height * PREVIEW_ORIGIN_Y_FRAC);

    return {
      x: (deviceX - originX) / px,
      y: (deviceY - originY) / px,
    };
  }

  function hitTestHandle(local: { x: number; y: number }, rect: Rect): CornerHandle | null {
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

  function hitTestBody(local: { x: number; y: number }, rect: Rect): boolean {
    const r = normalizeRect(rect);
    return r.x <= local.x && local.x <= r.x + r.w && r.y <= local.y && local.y <= r.y + r.h;
  }

  function ensureOverlaySize(): void {
    if (!overlayCanvasEl || !viewportEl) return;
    const rect = viewportEl.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return;
    const dpr = Math.max(1, window.devicePixelRatio || 1);
    const w = Math.max(1, Math.floor(rect.width * dpr));
    const h = Math.max(1, Math.floor(rect.height * dpr));

    if (overlayCanvasEl.width !== w || overlayCanvasEl.height !== h || lastDpr !== dpr) {
      overlayCanvasEl.width = w;
      overlayCanvasEl.height = h;
      lastDpr = dpr;
    }
  }

  function drawOverlay(): void {
    if (!overlayCanvasEl) return;
    const ctx = overlayCanvasEl.getContext("2d");
    if (!ctx) return;

    ensureOverlaySize();

    const w = overlayCanvasEl.width;
    const h = overlayCanvasEl.height;
    ctx.clearRect(0, 0, w, h);

    if (!move || message) return;

    const originX = Math.floor(w * 0.5);
    const originY = Math.floor(h * PREVIEW_ORIGIN_Y_FRAC);
    const px = lastDpr;

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

    // Draw hurtboxes under hitboxes.
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

  $effect(() => {
    void frameIndex;
    void move;
    void message;
    void editLayer;
    void editTool;
    void selection;
    void action;
    void draftRect;
    void liveOverride;
    ensureSelectionValid();
    drawOverlay();
  });

  const selectedEditor = $derived.by(() => {
    if (!move) return null;
    if (!selection) return null;
    const hb = getLayerArray(move, selection.layer)[selection.index] ?? null;
    if (!hb) return null;
    return { layer: selection.layer, index: selection.index, hb };
  });

  $effect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return;
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      if (tag === "input" || tag === "textarea" || tag === "select") return;

      if (e.key === "Escape") {
        selection = null;
        action = null;
        draftRect = null;
        liveOverride = null;
        return;
      }

      if (e.key === "Delete" || e.key === "Backspace") {
        if (!selection) return;
        e.preventDefault();
        deleteSelected();
        return;
      }

      if (e.key === "v" || e.key === "V") {
        editTool = "select";
        return;
      }

      if (e.key === "b" || e.key === "B") {
        editTool = "draw";
        selection = null;
        return;
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  function handlePointerDown(e: PointerEvent): void {
    if (!move || message) return;
    if (!overlayCanvasEl) return;
    if (e.button !== 0) return;

    const local = getLocalPt(e);
    if (!local) return;

    overlayCanvasEl.setPointerCapture(e.pointerId);

    if (editTool === "draw") {
      selection = null;
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
          selection = { layer: editLayer, index };
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
          selection = { layer: editLayer, index };
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

    selection = null;
    action = null;
  }

  function handlePointerMove(e: PointerEvent): void {
    if (!move || message) return;
    if (!action) return;
    if (e.pointerId !== action.pointerId) return;

    const local = getLocalPt(e);
    if (!local) return;

    if (action.kind === "draw") {
      draftRect = {
        x: action.startPt.x,
        y: action.startPt.y,
        w: local.x - action.startPt.x,
        h: local.y - action.startPt.y,
      };
      return;
    }

    if (action.kind === "move") {
      const dx = local.x - action.startPt.x;
      const dy = local.y - action.startPt.y;
      liveOverride = {
        layer: action.layer,
        index: action.index,
        rect: {
          x: action.startRect.x + dx,
          y: action.startRect.y + dy,
          w: action.startRect.w,
          h: action.startRect.h,
        },
      };
      return;
    }

    // resize
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

    liveOverride = {
      layer: action.layer,
      index: action.index,
      rect: {
        x: Math.min(local.x, fixed.x),
        y: Math.min(local.y, fixed.y),
        w: Math.abs(local.x - fixed.x),
        h: Math.abs(local.y - fixed.y),
      },
    };
  }

  function handlePointerUp(e: PointerEvent): void {
    if (!action) return;
    if (e.pointerId !== action.pointerId) return;

    if (overlayCanvasEl?.hasPointerCapture(e.pointerId)) {
      overlayCanvasEl.releasePointerCapture(e.pointerId);
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
      addRect(layer, nr);
      return;
    }

    if (action.kind === "move" || action.kind === "resize") {
      const rect =
        liveOverride && liveOverride.layer === action.layer && liveOverride.index === action.index
          ? liveOverride.rect
          : action.startRect;
      updateRect(action.layer, action.index, rect);
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

    if (overlayCanvasEl?.hasPointerCapture(e.pointerId)) {
      overlayCanvasEl.releasePointerCapture(e.pointerId);
    }
    action = null;
    draftRect = null;
    liveOverride = null;
  }

  $effect(() => {
    if (!viewportEl) return;
    resizeObs?.disconnect();
    const schedule = () => {
      if (resizeRaf) return;
      resizeRaf = requestAnimationFrame(() => {
        resizeRaf = 0;
        ensureCoreViewportSize();
        core?.renderOnce();
        drawOverlay();
      });
    };

    resizeObs = new ResizeObserver((entries) => {
      const cr = entries[0]?.contentRect;
      if (cr && (cr.width <= 0 || cr.height <= 0)) return;
      schedule();
    });
    resizeObs.observe(viewportEl);

    queueMicrotask(schedule);

    return () => {
      if (resizeRaf) cancelAnimationFrame(resizeRaf);
      resizeRaf = 0;
      resizeObs?.disconnect();
      resizeObs = null;
    };
  });

  onDestroy(() => {
    if (resizeRaf) cancelAnimationFrame(resizeRaf);
    resizeRaf = 0;
    core?.unmount();
    core = null;
    coreKey = null;
    coreActorError = null;
  });
</script>

<div class="preview-root">
  <div class="controls">
    <button class="btn" type="button" onclick={togglePlay} disabled={!!message}>
      {playing ? "Pause" : "Play"}
    </button>
    <button class="btn" type="button" onclick={() => step(-1)} disabled={!!message || frameIndex <= 0}>
      Prev
    </button>
    <button
      class="btn"
      type="button"
      onclick={() => step(1)}
      disabled={!!message || frameIndex >= effectiveTotal - 1}
    >
      Next
    </button>

    <div class="spacer"></div>

    <label class="inline">
      <span>Layer</span>
      <select
        value={editLayer}
        disabled={!!message}
        onchange={(e) => {
          editLayer = (e.currentTarget as HTMLSelectElement).value as Layer;
          selection = null;
          action = null;
          draftRect = null;
        }}
      >
        <option value="hitboxes">Hitboxes</option>
        <option value="hurtboxes">Hurtboxes</option>
      </select>
    </label>

    <label class="inline">
      <span>Tool</span>
      <select
        value={editTool}
        disabled={!!message}
        onchange={(e) => {
          editTool = (e.currentTarget as HTMLSelectElement).value as Tool;
          if (editTool === "draw") selection = null;
        }}
      >
        <option value="select">Select</option>
        <option value="draw">Draw</option>
      </select>
    </label>

    <label class="inline">
      <span>Speed</span>
      <select bind:value={speed} disabled={!!message}>
        {#each speedOptions as opt}
          <option value={opt}>{opt}x</option>
        {/each}
      </select>
    </label>
  </div>

  <div class="scrub">
    <input
      class="slider"
      type="range"
      min="0"
      max={effectiveTotal - 1}
      value={frameIndex}
      disabled={!!message}
      oninput={(e) => {
        frameIndex = clampFrame(parseInt((e.currentTarget as HTMLInputElement).value, 10));
      }}
    />
    <div class="frame-readout">f {frameIndex} / {effectiveTotal - 1}</div>
  </div>

  <div class="phase-bar" aria-hidden="true" title="Startup / Active / Recovery">
    <div class="seg startup" style={`flex:${phases.startup}`}></div>
    <div class="seg active" style={`flex:${phases.active}`}></div>
    <div class="seg recovery" style={`flex:${phases.recovery}`}></div>
    {#if phases.extra > 0}
      <div class="seg extra" style={`flex:${phases.extra}`}></div>
    {/if}
    <div class="marker" style={`left:${frameMarkerPct}%`}></div>
  </div>

  <div class="viewport" bind:this={viewportEl}>
    {#if message}
      <div class="message">{message}</div>
    {:else}
      {#if coreActorError}
        <div class="core-error">{coreActorError}</div>
      {/if}
      <canvas
        bind:this={overlayCanvasEl}
        class="hitbox-overlay"
        class:tool-draw={editTool === "draw"}
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        onpointercancel={handlePointerCancel}
      ></canvas>
    {/if}
  </div>

  {#if selectedEditor && selectedEditor.layer === editLayer}
    <div class="rect-editor">
      <div class="rect-row">
        <span class="rect-title">
          {selectedEditor.layer === "hitboxes" ? "Hitbox" : "Hurtbox"} #{selectedEditor.index + 1}
        </span>
        <button class="btn danger" type="button" onclick={deleteSelected} disabled={!!message}>Delete</button>
      </div>
      <div class="rect-row">
        <label class="inline">
          <span>Start</span>
          <input
            class="tiny"
            type="number"
            min="0"
            max={effectiveTotal - 1}
            value={selectedEditor.hb.frames?.[0] ?? 0}
            disabled={!!message}
            oninput={(e) => {
              const v = parseInt((e.currentTarget as HTMLInputElement).value, 10);
              updateFrames(selectedEditor.layer, selectedEditor.index, [v, selectedEditor.hb.frames?.[1] ?? v]);
            }}
          />
        </label>
        <label class="inline">
          <span>End</span>
          <input
            class="tiny"
            type="number"
            min="0"
            max={effectiveTotal - 1}
            value={selectedEditor.hb.frames?.[1] ?? 0}
            disabled={!!message}
            oninput={(e) => {
              const v = parseInt((e.currentTarget as HTMLInputElement).value, 10);
              updateFrames(selectedEditor.layer, selectedEditor.index, [selectedEditor.hb.frames?.[0] ?? v, v]);
            }}
          />
        </label>
        <div class="hint">Del: delete; Esc: clear; V: select; B: draw</div>
      </div>
    </div>
  {/if}
</div>

<style>
  .preview-root {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
  }

  .controls {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .btn {
    font-size: 12px;
    padding: 6px 10px;
    border-radius: 6px;
  }

  .spacer {
    flex: 1;
  }

  .inline {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .inline select {
    font-size: 12px;
  }

  .scrub {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 10px;
    align-items: center;
  }

  .phase-bar {
    position: relative;
    height: 8px;
    display: flex;
    border-radius: 999px;
    overflow: hidden;
    border: 1px solid var(--border);
    background: color-mix(in oklab, var(--bg-secondary) 80%, black 20%);
  }

  .seg {
    min-width: 0;
  }

  .seg.startup {
    background: rgba(148, 163, 184, 0.6);
  }

  .seg.active {
    background: rgba(239, 68, 68, 0.55);
  }

  .seg.recovery {
    background: rgba(59, 130, 246, 0.5);
  }

  .seg.extra {
    background: rgba(234, 179, 8, 0.35);
  }

  .marker {
    position: absolute;
    top: -1px;
    bottom: -1px;
    width: 2px;
    transform: translateX(-1px);
    background: rgba(250, 204, 21, 0.95);
    box-shadow: 0 0 0 1px rgba(17, 24, 39, 0.5);
    pointer-events: none;
  }

  .slider {
    width: 100%;
  }

  .frame-readout {
    font-family: monospace;
    font-size: 12px;
    color: var(--text-secondary);
    min-width: 96px;
    text-align: right;
  }

  .viewport {
    flex: 1;
    min-height: 240px;
    background: color-mix(in oklab, var(--bg-secondary) 92%, black 8%);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    position: relative;
  }

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

  .rect-editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: color-mix(in oklab, var(--bg-secondary) 92%, black 8%);
  }

  .rect-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .rect-title {
    font-size: 12px;
    font-family: monospace;
    color: var(--text-secondary);
  }

  .hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin-left: auto;
  }

  input.tiny {
    width: 76px;
  }

  .btn.danger {
    border-color: color-mix(in oklab, var(--accent) 25%, var(--border));
    color: var(--accent);
  }

  .message {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    z-index: 3;
  }

  .viewport :global(canvas.rendercore-canvas) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    z-index: 0;
    pointer-events: none;
  }

  .core-error {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    background: rgba(0, 0, 0, 0.18);
    pointer-events: none;
    z-index: 1;
  }
</style>
