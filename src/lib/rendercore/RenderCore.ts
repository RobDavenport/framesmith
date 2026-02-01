import type { GridHelper, OrthographicCamera, Scene, WebGLRenderer } from "three";

import type {
  ActorSpec,
  ActorStatus,
  ClockMode,
  SceneMode,
} from "./types";

import type { AssetProvider } from "./assets/TauriAssetProvider";
import { mergeActorStatus } from "./status";
import { PREVIEW_ORIGIN_Y_FRAC } from "./config";

type ThreeModule = typeof import("three");
type ActorKind = "sprite" | "gltf";

type ActorInstance = {
  readonly id: string;
  readonly kind: ActorKind;
  mount(scene: Scene): void;
  unmount(scene: Scene): void;
  update(spec: ActorSpec): void;
  getStatus(): ActorStatus;
};

type SpriteActorCtor = new (id: string, assets: AssetProvider) => ActorInstance;
type GltfActorCtor = new (id: string, assets: AssetProvider) => ActorInstance;

function formatError(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

export class RenderCore {
  private containerEl: HTMLElement | null = null;
  private renderer: WebGLRenderer | null = null;
  private scene: Scene | null = null;
  private camera: OrthographicCamera | null = null;

  private THREE: ThreeModule | null = null;
  private SpriteActor: SpriteActorCtor | null = null;
  private GltfActor: GltfActorCtor | null = null;
  private depsPromise: Promise<void> | null = null;
  private depsError: string | null = null;
  private mountSeq = 0;

  private grid: GridHelper | null = null;

  private clockMode: ClockMode = "manual";
  private sceneMode: SceneMode = "single";
  private rafHandle: number | null = null;

  private viewportW = 1;
  private viewportH = 1;
  private dpr = 1;

  private desiredActors: ActorSpec[] = [];
  private actors = new Map<string, ActorInstance>();
  private actorStatuses = new Map<string, ActorStatus>();

  constructor(private readonly assets: (actorId: string) => AssetProvider) {}

  getInitError(): string | null {
    return this.depsError;
  }

  mount(containerEl: HTMLElement): void {
    this.containerEl = containerEl;
    const seq = ++this.mountSeq;
    void this.ensureDepsLoaded().then(() => {
      if (this.mountSeq !== seq) return;
      this.initThree();
    });
  }

  unmount(): void {
    // Invalidate any in-flight init from an earlier mount().
    this.mountSeq++;

    this.stop();

    if (this.renderer?.domElement.parentElement) {
      this.renderer.domElement.parentElement.removeChild(this.renderer.domElement);
    }

    if (this.scene) {
      for (const actor of this.actors.values()) actor.unmount(this.scene);

      if (this.grid) {
        this.scene.remove(this.grid);
        this.grid.geometry?.dispose?.();
        const mat = (this.grid as unknown as { material?: unknown }).material;
        if (Array.isArray(mat)) mat.forEach((m) => (m as { dispose?: () => void }).dispose?.());
        else (mat as { dispose?: () => void } | undefined)?.dispose?.();
        this.grid = null;
      }
    }
    this.actors.clear();
    this.actorStatuses.clear();

    this.renderer?.dispose();
    this.renderer = null;
    this.scene = null;
    this.camera = null;
    this.containerEl = null;
  }

  setViewportSize(w: number, h: number, dpr: number): void {
    this.viewportW = Math.max(1, Math.floor(w));
    this.viewportH = Math.max(1, Math.floor(h));
    this.dpr = Math.max(0.5, dpr);

    if (this.renderer) {
      this.renderer.setPixelRatio(this.dpr);
      this.renderer.setSize(this.viewportW, this.viewportH, false);
    }
    if (this.camera) {
      this.camera.left = -this.viewportW / 2;
      this.camera.right = this.viewportW / 2;
      // Invert Y so +y is down (screen-like coordinates).
      this.camera.top = -this.viewportH / 2;
      this.camera.bottom = this.viewportH / 2;
      this.camera.updateProjectionMatrix();
      this.applySceneMode();
    }
  }

  setClockMode(mode: ClockMode): void {
    this.clockMode = mode;
  }

  setSceneMode(mode: SceneMode): void {
    this.sceneMode = mode;
    this.applySceneMode();
  }

  setActors(actors: ActorSpec[]): void {
    this.desiredActors = actors;
    this.reconcileActors();
  }

  getActorStatus(id: string): ActorStatus {
    return this.actorStatuses.get(id) ?? { loading: false, error: null };
  }

  renderOnce(): void {
    this.tickAndRender();
  }

  start(): void {
    if (this.clockMode !== "raf") return;
    if (this.rafHandle != null) return;
    if (typeof requestAnimationFrame !== "function") return;

    const loop = () => {
      this.rafHandle = requestAnimationFrame(loop);
      this.tickAndRender();
    };
    this.rafHandle = requestAnimationFrame(loop);
  }

  stop(): void {
    if (this.rafHandle != null && typeof cancelAnimationFrame === "function") {
      cancelAnimationFrame(this.rafHandle);
    }
    this.rafHandle = null;
  }

  private ensureDepsLoaded(): Promise<void> {
    if (this.THREE && this.SpriteActor && this.GltfActor) return Promise.resolve();
    if (this.depsPromise) return this.depsPromise;
    if (typeof window === "undefined") return Promise.resolve();
    if (this.depsError) return Promise.resolve();

    this.depsPromise = (async () => {
      const [three, spriteMod, gltfMod] = await Promise.all([
        import("three"),
        import("./actors/SpriteActor"),
        import("./actors/GltfActor"),
      ]);
      this.THREE = three;
      this.SpriteActor = spriteMod.SpriteActor as unknown as SpriteActorCtor;
      this.GltfActor = gltfMod.GltfActor as unknown as GltfActorCtor;
    })().catch((e) => {
      // Ensure callers never see an unhandled rejection.
      this.depsError = formatError(e);
    });

    return this.depsPromise;
  }

  private initThree(): void {
    const containerEl = this.containerEl;
    const THREE = this.THREE;
    if (!containerEl || !THREE) return;

    if (!this.scene) {
      this.scene = new THREE.Scene();
      this.scene.background = new THREE.Color(0x101217);
      const hemi = new THREE.HemisphereLight(0xffffff, 0x303040, 1);
      this.scene.add(hemi);
      const grid = new THREE.GridHelper(20, 20, 0x2a2f3a, 0x1f242d);
      grid.position.y = -1;
      this.scene.add(grid);
      this.grid = grid;
    }

    if (!this.camera) {
      this.camera = new THREE.OrthographicCamera(
        -this.viewportW / 2,
        this.viewportW / 2,
        -this.viewportH / 2,
        this.viewportH / 2,
        0.01,
        5000
      );
      this.applySceneMode();
    }

    if (!this.renderer) {
      this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
      this.renderer.setPixelRatio(this.dpr);
      this.renderer.setSize(this.viewportW, this.viewportH, false);

      // Ensure the canvas fills the mount element.
      const el = this.renderer.domElement;
      el.classList.add("rendercore-canvas");
      el.style.width = "100%";
      el.style.height = "100%";
      el.style.display = "block";
      el.style.pointerEvents = "none";
    }

    if (this.renderer.domElement.parentElement !== containerEl) {
      containerEl.appendChild(this.renderer.domElement);
    }

    // setActors() and setSceneMode() may have been called before mount().
    this.reconcileActors();
    this.applySceneMode();
  }

  private applySceneMode(): void {
    if (!this.camera) return;

    // Keep camera unrotated for 1:1 overlay alignment.
    this.camera.rotation.set(0, 0, 0);
    this.camera.position.z = 1000;

    if (this.sceneMode === "single") {
      this.camera.position.x = 0;
      // Align world origin with overlay origin at y = (PREVIEW_ORIGIN_Y_FRAC*h - h/2).
      const originYOffset = PREVIEW_ORIGIN_Y_FRAC * this.viewportH - this.viewportH / 2;
      this.camera.position.y = -originYOffset;
      return;
    }

    // Training mode keeps world origin centered (and tracks actors on X).
    this.camera.position.y = 0;
    this.updateTrainingCameraX();
  }

  private updateTrainingCameraX(): void {
    if (!this.camera) return;
    if (this.sceneMode !== "training") return;

    if (this.desiredActors.length === 0) {
      this.camera.position.x = 0;
      return;
    }

    let minX = Infinity;
    let maxX = -Infinity;
    for (const a of this.desiredActors) {
      const x = a.pos?.x;
      if (typeof x !== "number" || !Number.isFinite(x)) continue;
      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
    }

    if (!Number.isFinite(minX) || !Number.isFinite(maxX)) {
      this.camera.position.x = 0;
      return;
    }

    this.camera.position.x = (minX + maxX) / 2;
  }

  private reconcileActors(): void {
    if (!this.scene) return;
    if (!this.SpriteActor || !this.GltfActor) return;

    const wanted = new Set(this.desiredActors.map((a) => a.id));

    for (const [id, inst] of this.actors.entries()) {
      if (!wanted.has(id)) {
        inst.unmount(this.scene);
        this.actors.delete(id);
        this.actorStatuses.delete(id);
      }
    }

    for (const spec of this.desiredActors) {
      const existing = this.actors.get(spec.id);
      if (existing && existing.kind !== spec.visual.kind) {
        existing.unmount(this.scene);
        this.actors.delete(spec.id);
        this.actorStatuses.delete(spec.id);
      }

      if (this.actors.has(spec.id)) continue;

      const provider = this.assets(spec.id);
      const inst: ActorInstance =
        spec.visual.kind === "sprite"
          ? new this.SpriteActor(spec.id, provider)
          : new this.GltfActor(spec.id, provider);
      inst.mount(this.scene);
      this.actors.set(spec.id, inst);
      this.actorStatuses.set(spec.id, { loading: false, error: null });
    }
  }

  private tickAndRender(): void {
    if (!this.renderer || !this.scene || !this.camera) {
      void this.ensureDepsLoaded().then(() => this.initThree());
      return;
    }

    // Keep actor list consistent even if setActors() happened before mount().
    this.reconcileActors();

    const specById = new Map(this.desiredActors.map((a) => [a.id, a] as const));
    for (const [id, actor] of this.actors.entries()) {
      const spec = specById.get(id);
      if (!spec) continue;

      const prev = this.actorStatuses.get(id) ?? { loading: false, error: null };
      let updateError: string | null = null;

      try {
        actor.update(spec);
      } catch (e) {
        updateError = formatError(e);
      }

      let next: ActorStatus = prev;
      try {
        next = actor.getStatus();
      } catch (e) {
        next = { loading: false, error: formatError(e) };
      }

      this.actorStatuses.set(id, mergeActorStatus(prev, updateError, next));
    }

    if (this.sceneMode === "training") {
      this.updateTrainingCameraX();
    }

    this.renderer.render(this.scene, this.camera);
  }
}
