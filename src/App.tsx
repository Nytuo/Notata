import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Toaster } from "@/components/ui/sonner";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { AppBar } from "@/components/layout/AppBar";
import { Sidebar } from "@/components/layout/Sidebar";
import { StatusBar } from "@/components/layout/StatusBar";
import { FileList } from "@/components/library/FileList";
import { MetadataRouter } from "@/components/metadata/MetadataRouter";
import { useSettingsStore, watchSystemTheme } from "@/stores/settingsStore";

function App() {
  const { i18n } = useTranslation();
  const loadSettings = useSettingsStore((s) => s.load);
  const language = useSettingsStore((s) => s.language);

  useEffect(() => {
    // Theme is applied inside load(), before the first paint the user notices.
    loadSettings();
    return watchSystemTheme();
  }, [loadSettings]);

  useEffect(() => {
    if (language && i18n.language !== language) {
      i18n.changeLanguage(language);
    }
  }, [language, i18n]);

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
        <AppBar />

        <div className="flex flex-1 overflow-hidden">
          {/* Sizes carry explicit units: in react-resizable-panels v4 a bare
              number means pixels, not percent. Minimums are in pixels so each
              pane keeps a legible floor no matter the window size. */}
          <ResizablePanelGroup orientation="horizontal" className="flex-1">
            <ResizablePanel defaultSize="18%" minSize="170px" maxSize="340px">
              <Sidebar />
            </ResizablePanel>

            <ResizableHandle withHandle />

            <ResizablePanel defaultSize="42%" minSize="300px">
              <FileList />
            </ResizablePanel>

            <ResizableHandle withHandle />

            <ResizablePanel defaultSize="40%" minSize="340px">
              <MetadataRouter />
            </ResizablePanel>
          </ResizablePanelGroup>
        </div>

        <StatusBar />
      </div>
      <Toaster />
    </TooltipProvider>
  );
}

export default App;
