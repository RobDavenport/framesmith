export interface AssetProvider {
  readBase64(relativePath: string): Promise<string>;
  readArrayBuffer(relativePath: string): Promise<ArrayBuffer>;
  readDataUrl(relativePath: string, mimeType?: string): Promise<string>;
}

function base64ToArrayBuffer(b64: string): ArrayBuffer {
  // atob is available in browsers (Tauri webview). Avoid using Buffer.
  const bin = globalThis.atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes.buffer;
}

function extToMime(relativePath: string): string {
  const lower = relativePath.toLowerCase();
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
  if (lower.endsWith(".webp")) return "image/webp";
  if (lower.endsWith(".gif")) return "image/gif";
  if (lower.endsWith(".glb")) return "model/gltf-binary";
  if (lower.endsWith(".gltf")) return "model/gltf+json";
  return "application/octet-stream";
}

export class TauriAssetProvider implements AssetProvider {
  constructor(
    private readonly charactersDir: string,
    private readonly characterId: string
  ) {}

  async readBase64(relativePath: string): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<string>("read_character_asset_base64", {
      charactersDir: this.charactersDir,
      characterId: this.characterId,
      relativePath,
    });
  }

  async readArrayBuffer(relativePath: string): Promise<ArrayBuffer> {
    const b64 = await this.readBase64(relativePath);
    return base64ToArrayBuffer(b64);
  }

  async readDataUrl(relativePath: string, mimeType?: string): Promise<string> {
    const b64 = await this.readBase64(relativePath);
    const mime = mimeType ?? extToMime(relativePath);
    return `data:${mime};base64,${b64}`;
  }
}
