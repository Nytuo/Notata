import { open } from "@tauri-apps/plugin-dialog";
import { commands } from "@/lib/tauri";

export interface LocalImage {
  path: string;
  bytes: number[];
  base64: string;
  mimeType: string;
}

function mimeTypeForExtension(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "png":
      return "image/png";
    case "webp":
      return "image/webp";
    case "gif":
      return "image/gif";
    case "jpg":
    case "jpeg":
    default:
      return "image/jpeg";
  }
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
}

/// Prompts the user for a picture on disk and reads it into memory. Returns
/// `null` if the dialog was dismissed without a selection.
export async function pickLocalImage(): Promise<LocalImage | null> {
  const path = await open({
    multiple: false,
    directory: false,
    filters: [
      { name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "gif"] },
    ],
  });
  if (!path || typeof path !== "string") return null;

  const buffer = await commands.readFileBytes(path);
  const bytes = new Uint8Array(buffer);

  return {
    path,
    bytes: Array.from(bytes),
    base64: bytesToBase64(bytes),
    mimeType: mimeTypeForExtension(path),
  };
}
