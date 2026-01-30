<script lang="ts">
  import type { AnimationClip } from "$lib/types";
  import { readAssetBase64 } from "$lib/stores/assets.svelte";
  import { onMount, onDestroy } from "svelte";
  // NOTE: Three.js is loaded dynamically to avoid SSR build warnings and reduce baseline bundle size.

  type Props = {
    characterId: string;
    modelPath: string;
    clip: Extract<AnimationClip, { mode: "gltf" }>;
    frameIndex: number;
  };

  let { characterId, modelPath, clip, frameIndex }: Props = $props();

  let containerEl = $state<HTMLDivElement | null>(null);

  let ready = $state(false);

  let THREE: any = null;
  let GLTFLoaderCtor: any = null;

  let renderer: any = null;
  let scene: any = null;
  let camera: any = null;
  let pivotGroup: any = null;

  let mixer: any = null;
  let action: any = null;
  let activeClip: any = null;

  let loadError = $state<string | null>(null);
  let loading = $state(false);

  let resizeObs: ResizeObserver | null = null;
  let loadSeq = 0;

  function base64ToArrayBuffer(b64: string): ArrayBuffer {
    const binary = atob(b64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes.buffer;
  }

  function disposeObject3D(root: any): void {
    root.traverse((obj: any) => {
      const mesh = obj as any;
      const geom = (mesh as any).geometry as any;
      if (geom) geom.dispose();

      const mat = (mesh as any).material as any;
      if (mat) {
        const mats = Array.isArray(mat) ? mat : [mat];
        for (const m of mats) {
          const anyM = m as any;
          const texKeys = [
            "map",
            "normalMap",
            "metalnessMap",
            "roughnessMap",
            "emissiveMap",
            "aoMap",
            "alphaMap",
          ];
          for (const k of texKeys) {
            const t = anyM[k] as any;
            t?.dispose();
          }
          m.dispose();
        }
      }
    });
  }

  function renderNow(): void {
    if (!renderer || !scene || !camera) return;
    renderer.render(scene, camera);
  }

  function resize(): void {
    if (!renderer || !camera || !containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    const w = Math.max(1, Math.floor(rect.width));
    const h = Math.max(1, Math.floor(rect.height));
    renderer.setPixelRatio(Math.max(1, window.devicePixelRatio || 1));
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderNow();
  }

  onMount(() => {
    let cancelled = false;

    void (async () => {
      if (!containerEl) return;
      loading = true;
      loadError = null;

      try {
        THREE = await import("three");
        ({ GLTFLoader: GLTFLoaderCtor } = await import("three/examples/jsm/loaders/GLTFLoader.js"));
      } catch (e) {
        if (cancelled) return;
        loadError = e instanceof Error ? e.message : String(e);
        loading = false;
        return;
      }

      if (cancelled || !containerEl) return;

      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
      renderer.setClearColor(0x000000, 0);
      containerEl.appendChild(renderer.domElement);

      scene = new THREE.Scene();

      camera = new THREE.PerspectiveCamera(35, 1, 0.01, 100);
      camera.position.set(0, 1.4, 3.2);
      camera.lookAt(0, 1.0, 0);

      const hemi = new THREE.HemisphereLight(0xffffff, 0x223344, 0.9);
      scene.add(hemi);
      const dir = new THREE.DirectionalLight(0xffffff, 1.1);
      dir.position.set(2, 4, 2);
      scene.add(dir);

      pivotGroup = new THREE.Group();
      scene.add(pivotGroup);

      resizeObs = new ResizeObserver(() => resize());
      resizeObs.observe(containerEl);
      resize();

      ready = true;
      loading = false;
    })();

    return () => {
      cancelled = true;
      ready = false;

      resizeObs?.disconnect();
      resizeObs = null;

      if (pivotGroup) {
        disposeObject3D(pivotGroup);
        pivotGroup.clear();
      }

      mixer = null;
      action = null;
      activeClip = null;

      renderer?.dispose();
      if (renderer?.domElement?.parentElement) {
        renderer.domElement.parentElement.removeChild(renderer.domElement);
      }
      renderer = null;
      scene = null;
      camera = null;
      pivotGroup = null;
      THREE = null;
      GLTFLoaderCtor = null;
    };
  });

  $effect(() => {
    if (!ready) return;
    const seq = ++loadSeq;
    loadError = null;
    loading = true;

    if (!pivotGroup) {
      loading = false;
      return;
    }

    if (pivotGroup.children.length > 0) {
      for (const child of pivotGroup.children) disposeObject3D(child);
      pivotGroup.clear();
    }
    mixer = null;
    action = null;
    activeClip = null;

    void (async () => {
      try {
        const b64 = await readAssetBase64(characterId, modelPath);
        if (seq !== loadSeq) return;

        if (!GLTFLoaderCtor) {
          loadError = "3D preview not initialized";
          loading = false;
          return;
        }

        const loader = new GLTFLoaderCtor();
        const buf = base64ToArrayBuffer(b64);
        loader.parse(
          buf,
          "",
           (gltf: any) => {
            if (seq !== loadSeq) return;

            const root = gltf.scene;
            root.position.set(-clip.pivot.x, -clip.pivot.y, -clip.pivot.z);
            pivotGroup?.add(root);

             if (gltf.animations?.length) {
              const found = gltf.animations.find((a: any) => a.name === clip.clip) ?? null;
              activeClip = found;
              if (!activeClip) {
                loadError = `Animation clip not found in model: '${clip.clip}'`;
              } else {
                mixer = new THREE.AnimationMixer(root);
                action = mixer.clipAction(activeClip);
                action.setLoop(THREE.LoopOnce, 1);
                action.clampWhenFinished = true;
                action.play();
                action.paused = true;
              }
            } else {
              loadError = "Model has no animations";
            }

            loading = false;
            resize();
          },
           (err: any) => {
            if (seq !== loadSeq) return;
            loadError = err instanceof Error ? err.message : String(err);
            loading = false;
          }
        );
      } catch (e) {
        if (seq !== loadSeq) return;
        loadError = e instanceof Error ? e.message : String(e);
        loading = false;
      }
    })();
  });

  $effect(() => {
    if (!ready) return;
    if (!renderer || !scene || !camera) return;

    if (mixer && action && activeClip) {
      const t = Math.max(0, frameIndex / clip.fps);
      const clamped = Math.min(t, activeClip.duration);
      action.paused = true;
      action.time = clamped;
      mixer.update(0);
    }
    renderNow();
  });

  onDestroy(() => {
    loadSeq++;
  });
</script>

<div class="root" bind:this={containerEl}>
  {#if loading}
    <div class="overlay">Loading model...</div>
  {/if}
  {#if loadError}
    <div class="overlay">{loadError}</div>
  {/if}
</div>

<style>
  .root {
    position: absolute;
    inset: 0;
  }

  .overlay {
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
  }
</style>
