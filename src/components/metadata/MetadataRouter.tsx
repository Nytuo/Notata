import { useLibraryStore } from "@/stores/libraryStore";
import { MetadataPanel } from "@/components/metadata/MetadataPanel";
import { VideoMetadataPanel } from "@/components/video/VideoMetadataPanel";
import { BookMetadataPanel } from "@/components/book/BookMetadataPanel";

/**
 * Show the editor that matches the selected file.
 *
 * Routing keys off the file's own media type rather than the library root's
 * kind, so a stray video inside a music folder still gets the right editor.
 */
export function MetadataRouter() {
  const { files, selectedFileIds } = useLibraryStore();

  const selected = files.find((f) => f.id === selectedFileIds[0]);

  if (selected?.mediaType === "video") {
    return <VideoMetadataPanel />;
  }

  if (selected?.mediaType === "comic" || selected?.mediaType === "book") {
    return <BookMetadataPanel />;
  }

  return <MetadataPanel />;
}
