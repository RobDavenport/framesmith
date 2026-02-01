import * as THREE from "three";

import { createLoadSeq, type LoadSeq } from "../loadSeq";
import { shouldStartLoad } from "../loadDecision";
import { frameIndexToSeconds } from "../sampling";
import type { AssetProvider } from "../assets/TauriAssetProvider";
import type { ActorSpec, ActorStatus, ActorVisualGltf } from "../types";
import { isSupportedGltfModelPath } from "./gltfSupport";

type GLTFLoaderModule = typeof import("three/examples/jsm/loaders/GLTFLoader.js");

function formatError(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

export class GltfActor {
  readonly id: string;
  readonly kind = "gltf" as const;

  private readonly loadSeq: LoadSeq = createLoadSeq();
  private status: ActorStatus = { loading: false, error: null };

  private group: THREE.Group | null = null;
  private root: THREE.Object3D | null = null;
  private mixer: THREE.AnimationMixer | null = null;
  private action: THREE.AnimationAction | null = null;
  private animations: THREE.AnimationClip[] = [];

  private loadedModelPath: string | null = null;
  private inflightModelPath: string | null = null;
  private lastClipName: string | null = null;

  private gltfLoaderPromise: Promise<GLTFLoaderModule> | null = null;

  constructor(
    id: string,
    private readonly assets: AssetProvider
  ) {
    this.id = id;
  }

  getStatus(): ActorStatus {
    return this.status;
  }

  mount(scene: THREE.Scene): void {
    if (this.group) return;
    this.group = new THREE.Group();
    scene.add(this.group);
  }

  unmount(scene: THREE.Scene): void {
    // Invalidate any in-flight async loads.
    this.loadSeq.next();

    if (this.group) scene.remove(this.group);
    this.group = null;
    this.disposeModel();
    this.loadedModelPath = null;
    this.inflightModelPath = null;

    this.status = { loading: false, error: null };
  }

  update(spec: ActorSpec): void {
    if (!this.group) return;
    if (spec.visual.kind !== "gltf") return;

    this.applyTransform(spec);
    void this.ensureModelLoaded(spec.visual);
    this.applyPivot(spec.visual);
    this.sampleAnimation(spec.visual);
  }

  private applyTransform(spec: ActorSpec): void {
    if (!this.group) return;
    this.group.position.set(spec.pos.x, spec.pos.y, 0);
    // RenderCore uses a y-down camera; counter-flip glTF so it renders upright.
    this.group.scale.set(spec.facing === "right" ? 1 : -1, -1, 1);
  }

  private disposeModel(): void {
    if (this.mixer) this.mixer.stopAllAction();
    this.mixer = null;
    this.action = null;
    this.animations = [];

    if (this.root) this.disposeObject3D(this.root);

    if (this.root && this.group) this.group.remove(this.root);
    this.root = null;
    this.lastClipName = null;
  }

  private disposeObject3D(root: THREE.Object3D): void {
    root.traverse((obj) => {
      if (obj instanceof THREE.Mesh) {
        obj.geometry?.dispose?.();
        const mat = obj.material;
        const disposeMat = (m: THREE.Material) => {
          const anyMat = m as unknown as Record<string, unknown>;
          const disposeValue = (v: unknown): void => {
            if (v instanceof THREE.Texture) {
              v.dispose();
              return;
            }
            if (Array.isArray(v)) {
              for (const item of v) disposeValue(item);
            }
          };

          for (const v of Object.values(anyMat)) disposeValue(v);
          m.dispose();
        };
        if (Array.isArray(mat)) mat.forEach((m) => disposeMat(m));
        else if (mat) disposeMat(mat);
      }
    });
  }

  private sampleAnimation(visual: ActorVisualGltf): void {
    if (!this.mixer) return;
    const t = frameIndexToSeconds(visual.frameIndex, visual.clip.fps);
    this.mixer.setTime(t);
  }

  private applyPivot(visual: ActorVisualGltf): void {
    if (!this.root) return;
    const p = visual.clip.pivot;
    // Align the pivot point to the actor position.
    this.root.position.set(-p.x, -p.y, -p.z);
  }

  private async getGltfLoader(): Promise<GLTFLoaderModule["GLTFLoader"]> {
    if (!this.gltfLoaderPromise) {
      this.gltfLoaderPromise = import("three/examples/jsm/loaders/GLTFLoader.js");
    }
    const mod = await this.gltfLoaderPromise;
    return mod.GLTFLoader;
  }

  private async ensureModelLoaded(visual: ActorVisualGltf): Promise<void> {
    if (!this.group) return;

    const requestedPath = visual.modelPath;

    if (!isSupportedGltfModelPath(requestedPath)) {
      // Clear any previous model so the scene matches the requested visual.
      this.disposeModel();
      this.loadedModelPath = null;
      this.inflightModelPath = null;
      this.status = {
        loading: false,
        error: `Unsupported glTF model path (only .glb supported): ${requestedPath}`,
      };
      return;
    }

    if (this.loadedModelPath === requestedPath && this.root) {
      // Clip name might still change.
      if (this.lastClipName !== visual.clip.clip) {
        this.bindClip(visual);
      }
      return;
    }

    if (
      !shouldStartLoad({
        requestedPath,
        loadedPath: this.loadedModelPath,
        inflightPath: this.inflightModelPath,
      })
    ) {
      return;
    }

    const seq = this.loadSeq.next();
    this.inflightModelPath = requestedPath;
    this.status = { loading: true, error: null };

    try {
      const dataUrl = await this.assets.readDataUrl(requestedPath);
      if (!this.loadSeq.isCurrent(seq) || !this.group) return;

      const GLTFLoader = await this.getGltfLoader();
      if (!this.loadSeq.isCurrent(seq) || !this.group) return;

      const loader = new GLTFLoader();
      const gltf = await new Promise<{ scene: THREE.Object3D; animations: THREE.AnimationClip[] }>((resolve, reject) => {
        loader.load(dataUrl, (g) => resolve(g), undefined, (err) => reject(err));
      });

      if (!this.loadSeq.isCurrent(seq) || !this.group) {
        this.disposeObject3D(gltf.scene);
        return;
      }

      this.disposeModel();

      this.root = gltf.scene;
      this.animations = gltf.animations ?? [];
      this.group.add(this.root);
      this.mixer = new THREE.AnimationMixer(this.root);
      this.bindClip(visual);
      this.applyPivot(visual);

      this.loadedModelPath = requestedPath;
      if (this.inflightModelPath === requestedPath) this.inflightModelPath = null;

      this.status = { loading: false, error: null };
    } catch (e) {
      if (!this.loadSeq.isCurrent(seq)) return;
      if (this.inflightModelPath === requestedPath) this.inflightModelPath = null;
      this.status = { loading: false, error: formatError(e) };
    }
  }

  private bindClip(visual: ActorVisualGltf, animations?: THREE.AnimationClip[]): void {
    if (!this.mixer) return;
    const name = visual.clip.clip;
    this.lastClipName = name;

    const clips = animations ?? this.animations;
    const clip = THREE.AnimationClip.findByName(clips, name);
    if (!clip) {
      this.status = { loading: false, error: `Missing glTF animation clip: ${name}` };
      return;
    }

    if (this.action) {
      this.action.stop();
      this.action = null;
    }

    const action = this.mixer.clipAction(clip);
    action.play();
    action.paused = true;
    this.action = action;
  }
}
