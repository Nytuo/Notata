import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import {
  Settings,
  Check,
  ExternalLink,
  Loader2,
  Sun,
  Moon,
  Monitor,
  Languages,
  Palette,
  KeyRound,
  Info,
  Heart,
  ArrowDownCircle,
  AudioWaveform,
  FolderOpen,
  TriangleAlert,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useSettingsStore,
  THEME_ACCENTS,
  LANGUAGES,
  type ThemeMode,
} from "@/stores/settingsStore";
import { requestUpdateCheck } from "@/components/common/UpdaterModal";
import { commands } from "@/lib/tauri";
import { toast } from "sonner";
import type { ApiKeyStatus, FfmpegStatus } from "@/lib/types";

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const PROVIDERS = [
  {
    id: "tmdb",
    label: "TMDB",
    blurb: "Movies and TV series metadata, posters, and cast.",
    signupUrl: "https://www.themoviedb.org/settings/api",
    statusKey: "tmdbConfigured" as const,
  },
  {
    id: "tvdb",
    label: "TheTVDB",
    blurb: "Episode-level series data, useful where TMDB is thin.",
    signupUrl: "https://thetvdb.com/api-information",
    statusKey: "tvdbConfigured" as const,
  },
];

const MODES: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

/** Third-party work Notata is built on. */
const LIBRARIES = [
  { name: "Tauri", role: "Desktop runtime", license: "MIT / Apache-2.0" },
  { name: "React", role: "User interface", license: "MIT" },
  { name: "shadcn/ui", role: "Component library", license: "MIT" },
  { name: "Tailwind CSS", role: "Styling", license: "MIT" },
  { name: "lofty", role: "Audio tag read/write", license: "MIT / Apache-2.0" },
  { name: "quick-xml", role: "NFO and OPF parsing", license: "MIT" },
  { name: "zip", role: "CBZ and EPUB archives", license: "MIT" },
  { name: "rusqlite", role: "Local library index", license: "MIT" },
  { name: "FFmpeg", role: "Audio transcoding (external, not bundled)", license: "LGPL/GPL" },
  { name: "reqwest", role: "Provider HTTP calls", license: "MIT / Apache-2.0" },
  { name: "MusicBrainz", role: "Music metadata", license: "Community data" },
  { name: "TMDB / TheTVDB", role: "Video metadata", license: "API terms apply" },
];

const PREF_FFMPEG_PATH = "ffmpeg_path";

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const { i18n } = useTranslation();
  const [status, setStatus] = useState<ApiKeyStatus | null>(null);
  const [keys, setKeys] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [version, setVersion] = useState("");

  const [ffmpegPath, setFfmpegPath] = useState("");
  const [ffmpegStatus, setFfmpegStatus] = useState<FfmpegStatus | null>(null);
  const [checkingFfmpeg, setCheckingFfmpeg] = useState(false);
  const [savingFfmpeg, setSavingFfmpeg] = useState(false);

  const { mode, accent, language, setMode, setAccent, setLanguage } =
    useSettingsStore();

  useEffect(() => {
    if (!open) return;
    commands.getApiKeyStatus().then(setStatus).catch(() => {});
    invoke<string>("get_app_version").then(setVersion).catch(() => {});
    setKeys({});

    commands
      .getPreference(PREF_FFMPEG_PATH)
      .then((v) => setFfmpegPath(v ?? ""))
      .catch(() => {});
    checkFfmpeg();
  }, [open]);

  const checkFfmpeg = async () => {
    setCheckingFfmpeg(true);
    try {
      setFfmpegStatus(await commands.checkFfmpegAvailable());
    } catch {
      setFfmpegStatus({ available: false, version: null, path: "ffmpeg" });
    } finally {
      setCheckingFfmpeg(false);
    }
  };

  const handlePickFfmpeg = async () => {
    const selected = await openDialog({ directory: false, multiple: false });
    if (typeof selected === "string") setFfmpegPath(selected);
  };

  const handleSaveFfmpegPath = async () => {
    setSavingFfmpeg(true);
    try {
      await commands.setPreference(PREF_FFMPEG_PATH, ffmpegPath.trim());
      await checkFfmpeg();
      toast.success("ffmpeg path saved");
    } catch (e) {
      toast.error(`Could not save the ffmpeg path: ${e}`);
    } finally {
      setSavingFfmpeg(false);
    }
  };

  const handleSaveKey = async (provider: string) => {
    const value = keys[provider] ?? "";
    setSaving(provider);
    try {
      await commands.setApiKey(provider, value);
      setStatus(await commands.getApiKeyStatus());
      setKeys((k) => ({ ...k, [provider]: "" }));
      toast.success(
        value.trim()
          ? `${provider.toUpperCase()} key saved`
          : `${provider.toUpperCase()} key cleared`,
      );
    } catch (e) {
      toast.error(`Could not save the key: ${e}`);
    } finally {
      setSaving(null);
    }
  };

  const handleLanguage = async (next: string) => {
    await setLanguage(next);
    // i18next drives every translated string, so switch it in the same step.
    await i18n.changeLanguage(next);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[92vw] sm:max-w-2xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Settings className="h-5 w-5" />
            Settings
          </DialogTitle>
        </DialogHeader>

        <Tabs defaultValue="providers" className="flex min-h-0 flex-1 flex-col">
          <TabsList className="w-full">
            <TabsTrigger value="providers" className="flex-1 gap-1.5 text-xs">
              <KeyRound className="h-3.5 w-3.5" />
              Providers
            </TabsTrigger>
            <TabsTrigger value="appearance" className="flex-1 gap-1.5 text-xs">
              <Palette className="h-3.5 w-3.5" />
              Appearance
            </TabsTrigger>
            <TabsTrigger value="transcoding" className="flex-1 gap-1.5 text-xs">
              <AudioWaveform className="h-3.5 w-3.5" />
              Transcoding
            </TabsTrigger>
            <TabsTrigger value="about" className="flex-1 gap-1.5 text-xs">
              <Info className="h-3.5 w-3.5" />
              About
            </TabsTrigger>
          </TabsList>

          <div className="min-h-0 flex-1 overflow-y-auto pt-4">
            <TabsContent value="providers" className="mt-0 space-y-4">
              <p className="text-xs text-muted-foreground">
                Movie and series lookups need your own API key. Keys are stored
                locally and never leave this machine except to call the provider.
              </p>

              <Separator />

              {PROVIDERS.map((p) => {
                const configured = status?.[p.statusKey] ?? false;
                return (
                  <div key={p.id} className="space-y-2">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium">{p.label}</span>
                      {configured ? (
                        <Badge
                          variant="outline"
                          className="gap-1 border-emerald-500/40 bg-emerald-500/10 text-[10px] text-emerald-700 dark:text-emerald-400"
                        >
                          <Check className="h-3 w-3" />
                          Configured
                        </Badge>
                      ) : (
                        <Badge variant="outline" className="text-[10px]">
                          Not set
                        </Badge>
                      )}
                      <a
                        href={p.signupUrl}
                        target="_blank"
                        rel="noreferrer"
                        className="ml-auto flex items-center gap-1 text-[11px] text-muted-foreground hover:underline"
                      >
                        Get a key
                        <ExternalLink className="h-3 w-3" />
                      </a>
                    </div>

                    <p className="text-xs text-muted-foreground">{p.blurb}</p>

                    <div className="flex gap-2">
                      <Input
                        type="password"
                        className="h-8 flex-1 text-xs"
                        placeholder={
                          configured
                            ? "•••••••• (leave blank to keep)"
                            : "Paste your API key"
                        }
                        value={keys[p.id] ?? ""}
                        onChange={(e) =>
                          setKeys((k) => ({ ...k, [p.id]: e.target.value }))
                        }
                      />
                      <Button
                        size="sm"
                        className="h-8"
                        onClick={() => handleSaveKey(p.id)}
                        disabled={saving === p.id}
                      >
                        {saving === p.id ? (
                          <Loader2 className="h-3 w-3 animate-spin" />
                        ) : (
                          "Save"
                        )}
                      </Button>
                    </div>
                  </div>
                );
              })}
            </TabsContent>

            <TabsContent value="appearance" className="mt-0 space-y-5">
              <div className="space-y-2">
                <h3 className="text-sm font-medium">Theme</h3>
                <div className="flex gap-2">
                  {MODES.map((m) => {
                    const Icon = m.icon;
                    return (
                      <button
                        key={m.value}
                        onClick={() => setMode(m.value)}
                        aria-pressed={mode === m.value}
                        className={`flex flex-1 flex-col items-center gap-1.5 rounded-md border p-3 text-xs transition-colors hover:bg-accent ${
                          mode === m.value
                            ? "border-primary bg-accent"
                            : "text-muted-foreground"
                        }`}
                      >
                        <Icon className="h-4 w-4" />
                        {m.label}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="space-y-2">
                <h3 className="text-sm font-medium">Colour</h3>
                <div className="flex flex-wrap gap-2">
                  {THEME_ACCENTS.map((a) => (
                    <button
                      key={a.value}
                      onClick={() => setAccent(a.value)}
                      aria-pressed={accent === a.value}
                      className={`flex items-center gap-2 rounded-md border px-3 py-2 text-xs transition-colors hover:bg-accent ${
                        accent === a.value
                          ? "border-primary bg-accent"
                          : "text-muted-foreground"
                      }`}
                    >
                      <span
                        className="h-3.5 w-3.5 rounded-full border"
                        style={{ background: a.swatch }}
                      />
                      {a.label}
                      {accent === a.value && <Check className="h-3 w-3" />}
                    </button>
                  ))}
                </div>
              </div>

              <Separator />

              <div className="space-y-2">
                <h3 className="flex items-center gap-1.5 text-sm font-medium">
                  <Languages className="h-3.5 w-3.5" />
                  Language
                </h3>
                <Select value={language} onValueChange={handleLanguage}>
                  <SelectTrigger className="h-8 w-full text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {LANGUAGES.map((l) => (
                      <SelectItem key={l.value} value={l.value} className="text-xs">
                        {l.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-[11px] text-muted-foreground">
                  Translations are a work in progress — untranslated strings fall
                  back to English.
                </p>
              </div>
            </TabsContent>

            <TabsContent value="transcoding" className="mt-0 space-y-4">
              <p className="text-xs text-muted-foreground">
                Converting audio between formats (MP3, AAC, Ogg Vorbis, Opus,
                FLAC, ALAC, WAV, AIFF…) runs through ffmpeg, which isn't
                bundled with Notata. Install it and it'll be found on your
                PATH automatically, or point at a specific binary below.
              </p>

              <Separator />

              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">ffmpeg</span>
                  {checkingFfmpeg && (
                    <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
                  )}
                  {!checkingFfmpeg && ffmpegStatus?.available && (
                    <Badge
                      variant="outline"
                      className="gap-1 border-emerald-500/40 bg-emerald-500/10 text-[10px] text-emerald-700 dark:text-emerald-400"
                    >
                      <Check className="h-3 w-3" />
                      {ffmpegStatus.version ?? "Found"}
                    </Badge>
                  )}
                  {!checkingFfmpeg && !ffmpegStatus?.available && (
                    <Badge
                      variant="outline"
                      className="gap-1 border-destructive/40 bg-destructive/10 text-[10px] text-destructive"
                    >
                      <TriangleAlert className="h-3 w-3" />
                      Not found
                    </Badge>
                  )}
                </div>

                <p className="text-xs text-muted-foreground">
                  Leave blank to use "ffmpeg" from your PATH. ffprobe is
                  expected next to whatever binary you point at here.
                </p>

                <div className="flex gap-2">
                  <Input
                    className="h-8 flex-1 text-xs"
                    placeholder="/usr/local/bin/ffmpeg"
                    value={ffmpegPath}
                    onChange={(e) => setFfmpegPath(e.target.value)}
                  />
                  <Button size="sm" variant="outline" className="h-8" onClick={handlePickFfmpeg}>
                    <FolderOpen className="h-3.5 w-3.5" />
                  </Button>
                  <Button
                    size="sm"
                    className="h-8"
                    onClick={handleSaveFfmpegPath}
                    disabled={savingFfmpeg}
                  >
                    {savingFfmpeg ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      "Save"
                    )}
                  </Button>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="about" className="mt-0 space-y-4">
              <div className="flex items-start gap-3">
                <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                  <span className="text-xl font-semibold text-primary">N</span>
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <h3 className="text-base font-semibold">Notata</h3>
                    {version && (
                      <Badge variant="secondary" className="text-[10px]">
                        v{version}
                      </Badge>
                    )}
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">
                    A metadata manager for music, movies, series, comics, and
                    books. Reads and writes embedded tags, NFO sidecars, and the
                    XML inside CBZ and EPUB archives, with batch editing,
                    template renaming, and duplicate detection.
                  </p>
                </div>
              </div>

              <div className="flex items-center gap-2 rounded-md border p-3">
                <Heart className="h-4 w-4 shrink-0 text-primary" />
                <div className="min-w-0 text-xs">
                  <p className="font-medium">BEUX Arnaud (Nytuo)</p>
                  <p className="text-muted-foreground">
                    Author and maintainer
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="ml-auto h-7 gap-1 text-xs"
                  onClick={() => invoke("open_releases_page")}
                >
                  <ExternalLink className="h-3 w-3" />
                  GitHub
                </Button>
              </div>

              <div className="flex items-center gap-2 rounded-md border p-3">
                <ArrowDownCircle className="h-4 w-4 shrink-0 text-muted-foreground" />
                <div className="min-w-0 text-xs">
                  <p className="font-medium">Updates</p>
                  <p className="text-muted-foreground">
                    Check for a newer release.
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="ml-auto h-7 text-xs"
                  onClick={() => {
                    onOpenChange(false);
                    requestUpdateCheck();
                  }}
                >
                  Check now
                </Button>
              </div>

              <Separator />

              <div className="space-y-2">
                <h4 className="text-xs font-medium text-muted-foreground">
                  Built with
                </h4>
                <div className="rounded-md border">
                  {LIBRARIES.map((lib) => (
                    <div
                      key={lib.name}
                      className="flex items-center gap-2 border-b px-3 py-1.5 text-xs last:border-b-0"
                    >
                      <span className="w-28 shrink-0 font-medium">
                        {lib.name}
                      </span>
                      <span className="flex-1 truncate text-muted-foreground">
                        {lib.role}
                      </span>
                      <span className="shrink-0 text-[10px] text-muted-foreground">
                        {lib.license}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
