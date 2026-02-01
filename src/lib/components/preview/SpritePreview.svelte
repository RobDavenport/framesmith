<script lang="ts">
  import type { AnimationClip } from "$lib/types";
  import { readAssetBase64 } from "$lib/stores/assets.svelte";
  import { onDestroy } from "svelte";
  import { PREVIEW_ORIGIN_Y_FRAC } from "$lib/rendercore/config";

  type Props = {
    characterId: string;
    texturePath: string;
    clip: Extract<AnimationClip, { mode: "sprite" }>;
    frameIndex: number;
  };

  let { characterId, texturePath, clip, frameIndex }: Props = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let containerEl = $state<HTMLDivElement | null>(null);

  let img = $state<HTMLImageElement | null>(null);
  let loadError = $state<string | null>(null);

  let resizeObs: ResizeObserver | null = null;
  let lastDpr = 1;

  function extToMime(path: string): string {
    const lower = path.toLowerCase();
    if (lower.endsWith(".png")) return "image/png";
    if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
    if (lower.endsWith(".webp")) return "image/webp";
    if (lower.endsWith(".gif")) return "image/gif";
    return "application/octet-stream";
  }

  function base64ToDataUrl(b64: string, mime: string): string {
    if (mime.startsWith("image/")) return `data:${mime};base64,${b64}`;
    return `data:image/png;base64,${b64}`;
  }

  function ensureCanvasSize(): void {
    if (!canvasEl || !containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    const dpr = Math.max(1, window.devicePixelRatio || 1);

    const w = Math.max(1, Math.floor(rect.width * dpr));
    const h = Math.max(1, Math.floor(rect.height * dpr));
    if (canvasEl.width !== w || canvasEl.height !== h || lastDpr !== dpr) {
      canvasEl.width = w;
      canvasEl.height = h;
      lastDpr = dpr;
    }
  }

  function draw(): void {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    ensureCanvasSize();

    const w = canvasEl.width;
    const h = canvasEl.height;
    ctx.clearRect(0, 0, w, h);

    // Background grid.
    ctx.fillStyle = "rgba(0,0,0,0.18)";
    ctx.fillRect(0, 0, w, h);
    ctx.strokeStyle = "rgba(255,255,255,0.06)";
    ctx.lineWidth = 1;
    const grid = 32 * lastDpr;
    for (let x = 0; x < w; x += grid) {
      ctx.beginPath();
      ctx.moveTo(x + 0.5, 0);
      ctx.lineTo(x + 0.5, h);
      ctx.stroke();
    }
    for (let y = 0; y < h; y += grid) {
      ctx.beginPath();
      ctx.moveTo(0, y + 0.5);
      ctx.lineTo(w, y + 0.5);
      ctx.stroke();
    }

    const originX = Math.floor(w * 0.5);
    const originY = Math.floor(h * PREVIEW_ORIGIN_Y_FRAC);
    ctx.strokeStyle = "rgba(255,255,255,0.25)";
    ctx.beginPath();
    ctx.moveTo(0, originY + 0.5);
    ctx.lineTo(w, originY + 0.5);
    ctx.stroke();

    ctx.fillStyle = "rgba(255,255,255,0.4)";
    ctx.fillRect(originX - 2, originY - 2, 4, 4);

    if (!img) return;

    const frameW = clip.frame_size.w;
    const frameH = clip.frame_size.h;
    const cols = Math.max(1, Math.floor(img.width / frameW));
    const maxFrame = Math.max(0, clip.frames - 1);
    const f = Math.max(0, Math.min(maxFrame, frameIndex));
    const srcX = (f % cols) * frameW;
    const srcY = Math.floor(f / cols) * frameH;

    const dx = originX - clip.pivot.x * lastDpr;
    const dy = originY - clip.pivot.y * lastDpr;
    const dw = frameW * lastDpr;
    const dh = frameH * lastDpr;

    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(img, srcX, srcY, frameW, frameH, dx, dy, dw, dh);
  }

  let loadSeq = 0;
  $effect(() => {
    const seq = ++loadSeq;
    img = null;
    loadError = null;

    const mime = extToMime(texturePath);
    void (async () => {
      try {
        const b64 = await readAssetBase64(characterId, texturePath);
        if (seq !== loadSeq) return;

        const nextImg = new Image();
        nextImg.onload = () => {
          if (seq !== loadSeq) return;
          img = nextImg;
          draw();
        };
        nextImg.onerror = () => {
          if (seq !== loadSeq) return;
          loadError = "Failed to decode texture image";
        };
        nextImg.src = base64ToDataUrl(b64, mime);
      } catch (e) {
        if (seq !== loadSeq) return;
        loadError = e instanceof Error ? e.message : String(e);
      }
    })();
  });

  $effect(() => {
    void frameIndex;
    void clip;
    draw();
  });

  $effect(() => {
    if (!containerEl) return;
    resizeObs?.disconnect();
    resizeObs = new ResizeObserver(() => draw());
    resizeObs.observe(containerEl);
    return () => resizeObs?.disconnect();
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    resizeObs = null;
    loadSeq++;
  });
</script>

<div class="root" bind:this={containerEl}>
  <canvas bind:this={canvasEl} class="canvas"></canvas>
  {#if loadError}
    <div class="overlay">{loadError}</div>
  {/if}
</div>

<style>
  .root {
    position: absolute;
    inset: 0;
  }

  .canvas {
    width: 100%;
    height: 100%;
    display: block;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    color: var(--text-secondary);
    font-size: 13px;
    text-align: center;
    background: rgba(0, 0, 0, 0.25);
  }
</style>
