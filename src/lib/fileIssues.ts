import type { MediaFile } from "@/lib/types";

/**
 * Which files the "Problems" filter shows. Codes match
 * `metadata::reader::issue` on the backend.
 */
export type IssueFilter =
  | "off"
  | "any"
  | "playback"
  | "missing_cover"
  | "missing_title"
  | "missing_artist"
  | "missing_album"
  | "missing_track";

/** Problems that can stop a file from playing at all. */
const PLAYBACK_ISSUES = ["unreadable", "no_duration"];

/** `no_tags` implies every field is missing, so it counts toward each. */
const IMPLIES_MISSING = ["no_tags"];

export const ISSUE_FILTERS: {
  value: Exclude<IssueFilter, "off">;
  label: string;
  hint: string;
}[] = [
  {
    value: "any",
    label: "Any problem",
    hint: "Files with anything below",
  },
  {
    value: "playback",
    label: "Won't play",
    hint: "Unreadable file or zero-length audio",
  },
  {
    value: "missing_cover",
    label: "Missing cover",
    hint: "No embedded artwork",
  },
  {
    value: "missing_title",
    label: "Missing title",
    hint: "Servers fall back to the filename",
  },
  {
    value: "missing_artist",
    label: "Missing artist",
    hint: "Ends up under Unknown Artist",
  },
  {
    value: "missing_album",
    label: "Missing album",
    hint: "Ends up under Unknown Album, or is not grouped",
  },
  {
    value: "missing_track",
    label: "Missing track number",
    hint: "Album order falls back to filename",
  },
];

/** Human-readable label for one issue code, for the row tooltip. */
export const ISSUE_LABELS: Record<string, string> = {
  unreadable: "File could not be read",
  no_duration: "Zero-length audio",
  no_tags: "No tags at all",
  missing_title: "No title",
  missing_artist: "No artist",
  missing_album: "No album",
  missing_track: "No track number",
  missing_cover: "No cover art",
};

export function matchesIssueFilter(file: MediaFile, filter: IssueFilter): boolean {
  if (filter === "off") return true;
  const issues = file.issues;
  if (!issues) return false;

  switch (filter) {
    case "any":
      return issues.length > 0;
    case "playback":
      return issues.some((i) => PLAYBACK_ISSUES.includes(i));
    default:
      return (
        issues.includes(filter) ||
        (filter !== "missing_cover" && issues.some((i) => IMPLIES_MISSING.includes(i)))
      );
  }
}

/** Audio files that have never been analysed and need a rescan. */
export function unanalysedAudioCount(files: MediaFile[]): number {
  return files.filter((f) => f.mediaType === "audio" && f.issues === null).length;
}
