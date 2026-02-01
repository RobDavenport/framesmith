import * as THREE from "three";

import { createLoadSeq, type LoadSeq } from "../loadSeq";
import { shouldStartLoad } from "../loadDecision";
import { clampSpriteFrame, spriteSheetFrameUv } from "../sampling";
import type { AssetProvider } from "../assets/TauriAssetProvider";
import type { ActorSpec, ActorStatus, ActorVisualSprite } from "../types";

function formatError(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

export class SpriteActor {
  readonly id: string;
  readonly kind = "sprite" as const;

  private readonly loadSeq: LoadSeq = createLoadSeq();
  private status: ActorStatus = { loading: false, error: null };

  private group: THREE.Group | null = null;
  private mesh: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial> | null = null;
  private texture: THREE.Texture | null = null;
  private textureImageSize: { w: number; h: number } | null = null;

  private loadedTexturePath: string | null = null;
  private inflightTexturePath: string | null = null;
  private lastFrameSizeKey: string | null = null;

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

    const geom = new THREE.PlaneGeometry(1, 1);
    const mat = new THREE.MeshBasicMaterial({
      transparent: true,
      depthTest: true,
    });
    this.mesh = new THREE.Mesh(geom, mat);
    this.group.add(this.mesh);
  }

  unmount(scene: THREE.Scene): void {
    // Invalidate any in-flight async loads.
    this.loadSeq.next();

    if (this.group) scene.remove(this.group);
    this.group = null;

    if (this.mesh) {
      this.mesh.geometry.dispose();
      this.mesh.material.dispose();
    }
    this.mesh = null;

    if (this.texture) this.texture.dispose();
    this.texture = null;
    this.textureImageSize = null;
    this.loadedTexturePath = null;
    this.inflightTexturePath = null;

    this.status = { loading: false, error: null };
  }

  update(spec: ActorSpec): void {
    if (!this.group || !this.mesh) return;
    if (spec.visual.kind !== "sprite") return;

    this.applyTransform(spec);
    void this.ensureTextureLoaded(spec.visual);
    this.applySpriteFrame(spec.visual);
  }

  private applyTransform(spec: ActorSpec): void {
    if (!this.group) return;
    this.group.position.set(spec.pos.x, spec.pos.y, 0);
    this.group.scale.x = spec.facing === "right" ? 1 : -1;
  }

  private applySpriteFrame(visual: ActorVisualSprite): void {
    if (!this.mesh?.material.map) return;
    const frames = visual.clip.frames;
    const idx = clampSpriteFrame(visual.frameIndex, frames);

    const map = this.mesh.material.map;

    const img = this.textureImageSize;
    if (img) {
      const uv = spriteSheetFrameUv({
        frameIndex: idx,
        frames,
        frameW: visual.clip.frame_size.w,
        frameH: visual.clip.frame_size.h,
        imageW: img.w,
        imageH: img.h,
      });
      map.repeat.set(uv.repeatX, uv.repeatY);
      map.offset.set(uv.offsetX, uv.offsetY);
      return;
    }

    // Fallback: treat as a single-row spritesheet.
    const u = frames > 0 ? 1 / frames : 1;
    map.repeat.set(u, 1);
    map.offset.set(idx * u, 0);
  }

  private resizeQuad(visual: ActorVisualSprite): void {
    if (!this.mesh) return;
    const w = visual.clip.frame_size.w;
    const h = visual.clip.frame_size.h;

    // RenderCore uses pixel units, so the quad is sized in pixels.
    this.mesh.scale.set(w, h, 1);

    // Pivot is in pixels from top-left (asset convention). Offset mesh so the
    // pivot point lands on the actor position (group origin).
    const px = visual.clip.pivot.x;
    const py = visual.clip.pivot.y;
    // Y is screen-like (+y down).
    this.mesh.position.set(w / 2 - px, h / 2 - py, 0);
  }

  private async ensureTextureLoaded(visual: ActorVisualSprite): Promise<void> {
    if (!this.mesh) return;

    const frameKey = `${visual.clip.frame_size.w}x${visual.clip.frame_size.h}:${visual.clip.pivot.x},${visual.clip.pivot.y}`;
    if (this.lastFrameSizeKey !== frameKey) {
      this.lastFrameSizeKey = frameKey;
      this.resizeQuad(visual);
    }

    const requestedPath = visual.texturePath;
    if (
      !shouldStartLoad({
        requestedPath,
        loadedPath: this.loadedTexturePath,
        inflightPath: this.inflightTexturePath,
      })
    ) {
      return;
    }

    const seq = this.loadSeq.next();
    this.inflightTexturePath = requestedPath;
    this.status = { loading: true, error: null };

    try {
      const dataUrl = await this.assets.readDataUrl(requestedPath);
      if (!this.loadSeq.isCurrent(seq) || !this.group || !this.mesh) return;

      const loader = new THREE.TextureLoader();
      const tex = await new Promise<THREE.Texture>((resolve, reject) => {
        loader.load(
          dataUrl,
          (t) => resolve(t),
          undefined,
          (err) => reject(err)
        );
      });

      if (!this.loadSeq.isCurrent(seq) || !this.group || !this.mesh) {
        tex.dispose();
        return;
      }

      if (this.texture) this.texture.dispose();
      this.texture = tex;

      // Cache image dimensions for UV sampling (multi-row spritesheets).
      const img = tex.image as { width?: unknown; height?: unknown } | undefined;
      const w = typeof img?.width === "number" ? img.width : null;
      const h = typeof img?.height === "number" ? img.height : null;
      this.textureImageSize = w != null && h != null ? { w, h } : null;

      tex.flipY = false;
      tex.wrapS = THREE.RepeatWrapping;
      tex.wrapT = THREE.RepeatWrapping;
      tex.needsUpdate = true;

      this.mesh.material.map = tex;
      this.mesh.material.needsUpdate = true;

      this.loadedTexturePath = requestedPath;
      if (this.inflightTexturePath === requestedPath) this.inflightTexturePath = null;
      this.status = { loading: false, error: null };
    } catch (e) {
      if (!this.loadSeq.isCurrent(seq)) return;
      if (this.inflightTexturePath === requestedPath) this.inflightTexturePath = null;
      this.status = { loading: false, error: formatError(e) };
    }
  }
}
