<script lang="ts">
  import { onDestroy } from "svelte";

  import { RenderCore } from "$lib/rendercore/RenderCore";
  import { TauriAssetProvider } from "$lib/rendercore/assets/TauriAssetProvider";
  import type { ActorSpec } from "$lib/rendercore/types";

  interface Props {
    charactersDir: string;
    characterId: string;
    actors: ActorSpec[];
    error?: string | null;
  }

  let { charactersDir, characterId, actors, error = null }: Props = $props();

  let viewportEl = $state<HTMLDivElement | null>(null);

  let core = $state<RenderCore | null>(null);
  let coreKey = $state<string | null>(null);
  let resizeObs: ResizeObserver | null = null;
  let resizeRaf: number | null = null;

  let coreErrors = $state<string[]>([]);

  function ensureCoreViewportSize(): boolean {
    if (!core || !viewportEl) return false;
    if (typeof window === "undefined") return false;

    const rect = viewportEl.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) return false;

    const dpr = Math.max(1, window.devicePixelRatio || 1);
    core.setViewportSize(rect.width, rect.height, dpr);
    return true;
  }

  function scheduleResizeRender(): void {
    if (typeof window === "undefined") return;
    if (resizeRaf !== null) return;
    resizeRaf = window.requestAnimationFrame(() => {
      resizeRaf = null;
      const sized = ensureCoreViewportSize();
      if (sized) core?.renderOnce();
    });
  }

  $effect(() => {
    const host = viewportEl;
    const dir = charactersDir?.trim();
    const id = characterId?.trim();

    if (!host || !dir || !id) {
      coreErrors = [];
      core?.unmount();
      core = null;
      coreKey = null;
      return;
    }

    const key = `${dir}::${id}`;
    if (!core || coreKey !== key) {
      core?.unmount();

      const next = new RenderCore(() => new TauriAssetProvider(dir, id));
      next.setClockMode("manual");
      next.setSceneMode("training");
      next.mount(host);

      core = next;
      coreKey = key;
    }

    scheduleResizeRender();
  });

  $effect(() => {
    if (!core) return;
    core.setActors(actors ?? []);
    const sized = ensureCoreViewportSize();
    if (sized) core.renderOnce();

    const initError = core.getInitError();
    if (initError) {
      coreErrors = [`RenderCore init error: ${initError}`];
      return;
    }

    const errs: string[] = [];
    for (const actor of actors ?? []) {
      const status = core.getActorStatus(actor.id);
      if (status.error) errs.push(`${actor.id}: ${status.error}`);
    }
    coreErrors = errs;
  });

  $effect(() => {
    if (!viewportEl) return;
    resizeObs?.disconnect();
    if (typeof ResizeObserver !== "undefined") {
      resizeObs = new ResizeObserver(() => scheduleResizeRender());
      resizeObs.observe(viewportEl);
    }

    scheduleResizeRender();

    return () => {
      resizeObs?.disconnect();
      resizeObs = null;
    };
  });

  onDestroy(() => {
    core?.unmount();
    core = null;
    coreKey = null;
    if (resizeRaf !== null && typeof window !== "undefined") {
      window.cancelAnimationFrame(resizeRaf);
      resizeRaf = null;
    }
    resizeObs?.disconnect();
    resizeObs = null;
  });
</script>

<div class="viewport" bind:this={viewportEl}>
  {#if error}
    <div class="overlay">{error}</div>
  {:else if coreErrors.length > 0}
    <div class="overlay">{coreErrors.join("\n")}</div>
  {/if}
</div>

<style>
  .viewport {
    position: relative;
    width: 100%;
    height: 100%;
    background: color-mix(in oklab, var(--bg-secondary) 92%, black 8%);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }

  .viewport :global(canvas.rendercore-canvas) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    pointer-events: none;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    white-space: pre-wrap;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    background: rgba(0, 0, 0, 0.18);
    pointer-events: none;
    z-index: 1;
  }
</style>
