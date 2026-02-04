<script lang="ts">
  import type { AnimationClip, CharacterAssets, FrameHitbox, State, Rect } from "$lib/types";
  import { onDestroy } from "svelte";

  import { getProjectPath } from "$lib/stores/project.svelte";
  import { RenderCore } from "$lib/rendercore/RenderCore";
  import { TauriAssetProvider } from "$lib/rendercore/assets/TauriAssetProvider";
  import { buildActorSpec } from "$lib/rendercore/buildActorSpec";
  import type { ActorSpec } from "$lib/rendercore/types";
  import BoxEditor from "./preview/BoxEditor.svelte";
  import { type Layer, snapRect } from "./preview/useBoxDrag";

  type Tool = "select" | "draw";
  type Selection = { layer: Layer; index: number };

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

  let core = $state<RenderCore | null>(null);
  let coreKey = $state<string | null>(null);
  let coreActorError = $state<string | null>(null);

  const ACTOR_ID = "p1";

  let editLayer = $state<Layer>("hitboxes");
  let editTool = $state<Tool>("select");
  let selection = $state<Selection | null>(null);

  let boxEditorRef = $state<BoxEditor | null>(null);

  let resizeObs: ResizeObserver | null = null;
  let resizeRaf = 0;
  let lastDpr = $state(1);

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

  function ensureOverlaySize(): void {
    const overlayCanvasEl = boxEditorRef?.getCanvas();
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
    const overlayCanvasEl = boxEditorRef?.getCanvas();
    if (!overlayCanvasEl) return;
    const ctx = overlayCanvasEl.getContext("2d");
    if (!ctx) return;

    ensureOverlaySize();

    const w = overlayCanvasEl.width;
    const h = overlayCanvasEl.height;
    ctx.clearRect(0, 0, w, h);

    boxEditorRef?.drawOverlay(ctx);
  }

  $effect(() => {
    void frameIndex;
    void move;
    void message;
    void editLayer;
    void editTool;
    void selection;
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
        }}
      >
        <option value="hitboxes">Hitboxes</option>
        <option value="hurtboxes">Hurtboxes</option>
        <option value="pushboxes">Pushboxes</option>
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
      <BoxEditor
        bind:this={boxEditorRef}
        {move}
        {editLayer}
        {editTool}
        {frameIndex}
        dpr={lastDpr}
        canvasWidth={boxEditorRef?.getCanvas()?.width ?? 0}
        canvasHeight={boxEditorRef?.getCanvas()?.height ?? 0}
        {message}
        bind:selection
        onSelectionChange={(sel) => {
          selection = sel;
        }}
        onRectUpdate={updateRect}
        onRectAdd={addRect}
      />
    {/if}
  </div>

  {#if selectedEditor && selectedEditor.layer === editLayer}
    <div class="rect-editor">
      <div class="rect-row">
        <span class="rect-title">
          {selectedEditor.layer === "hitboxes" ? "Hitbox" : selectedEditor.layer === "hurtboxes" ? "Hurtbox" : "Pushbox"} #{selectedEditor.index + 1}
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
