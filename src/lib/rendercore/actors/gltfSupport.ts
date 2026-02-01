// glTF support helpers.
//
// We currently only support binary .glb. JSON .gltf often references external
// buffers/textures; our asset loading path is currently single-file (data URL)
// and does not resolve additional relative fetches.

function stripQueryAndHash(path: string): string {
  const idxQ = path.indexOf("?");
  const idxH = path.indexOf("#");
  const cut =
    idxQ === -1
      ? idxH
      : idxH === -1
        ? idxQ
        : Math.min(idxQ, idxH);
  return cut === -1 ? path : path.slice(0, cut);
}

export function isSupportedGltfModelPath(modelPath: string): boolean {
  const base = stripQueryAndHash(modelPath).toLowerCase();
  return base.endsWith(".glb");
}
