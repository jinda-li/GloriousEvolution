import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { HistoryView } from "./components/HistoryView";
import { HomeDashboard } from "./components/HomeDashboard";
import { RecorderApp } from "./components/RecorderApp";
import { SettingsView } from "./components/SettingsView";
import { Sidebar, type View } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { DEFAULT_SETTINGS } from "./constants";
import { I18nProvider, resolveLocale } from "./i18n";
import type {
  AppInfo,
  AppSettings,
  HistoryStore,
  LatencyMetricsEvent,
  ProcessResult,
  RecordingEvent,
  RecordingStatus,
} from "./types";

export type SaveState = { kind: "idle" | "saving" | "saved" | "error"; message?: string };

const EMPTY_HISTORY: HistoryStore = {
  stats: { totalSeconds: 0, totalWords: 0, totalSessions: 0 },
  entries: [],
};

function App() {
  const isRecorderWindow = useMemo(() => window.location.pathname === "/recorder", []);
  return isRecorderWindow ? <RecorderApp /> : <MainApp />;
}

function MainApp() {
  const [view, setView] = useState<View>(() => initialView());
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loaded, setLoaded] = useState(false);
  const [saveState, setSaveState] = useState<SaveState>({ kind: "idle" });
  const [appInfo, setAppInfo] = useState<AppInfo>({ version: "", defaultSystemPrompt: "" });
  const [history, setHistory] = useState<HistoryStore>(EMPTY_HISTORY);
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus>("idle");
  const [statusMessage, setStatusMessage] = useState("");
  const [lastResult, setLastResult] = useState<ProcessResult | null>(null);
  const [latency, setLatency] = useState<LatencyMetricsEvent | null>(null);
  const skipNextSave = useRef(true);
  const locale = resolveLocale(settings.uiLanguage);

  // The backend localizes errors, progress and the tray menu; keep it in step.
  useEffect(() => {
    document.documentElement.lang = locale;
    if (loaded) {
      void invoke("set_locale", { locale });
    }
  }, [locale, loaded]);

  useEffect(() => {
    void Promise.all([
      invoke<AppSettings>("load_settings"),
      invoke<AppInfo>("app_info"),
      invoke<HistoryStore>("get_history"),
    ])
      .then(([loadedSettings, info, store]) => {
        setSettings({ ...DEFAULT_SETTINGS, ...loadedSettings });
        setAppInfo(info);
        setHistory(store);
        setLoaded(true);
      })
      .catch((error) => setStatusMessage(String(error)));

    const disposers = [
      listen<RecordingEvent>("recording-status", (event) => {
        setRecordingStatus(event.payload.status);
        setStatusMessage(event.payload.message);
      }),
      listen<ProcessResult>("process-result", (event) => setLastResult(event.payload)),
      listen<HistoryStore>("history-updated", (event) => setHistory(event.payload)),
      listen<LatencyMetricsEvent>("latency-metrics", (event) => setLatency(event.payload)),
    ];
    return () => disposers.forEach((dispose) => void dispose.then((fn) => fn()));
  }, []);

  const persist = useCallback(async (next: AppSettings) => {
    setSaveState({ kind: "saving" });
    try {
      await invoke("save_settings", { newSettings: next });
      await emit("settings-changed", next);
      setSaveState({ kind: "saved" });
    } catch (error) {
      setSaveState({ kind: "error", message: String(error) });
    }
  }, []);

  // Autosave: settings changes are persisted after a short pause in typing.
  useEffect(() => {
    if (!loaded) {
      return;
    }
    if (skipNextSave.current) {
      skipNextSave.current = false;
      return;
    }
    const timer = window.setTimeout(() => void persist(settings), 500);
    return () => window.clearTimeout(timer);
  }, [settings, loaded, persist]);

  async function toggleRecording() {
    try {
      await invoke("toggle_recording");
    } catch (error) {
      setRecordingStatus("error");
      setStatusMessage(String(error));
    }
  }

  const needsSetup = loaded && !settings.apiKey.trim();

  return (
    <I18nProvider locale={locale}>
      <div className="app-shell">
        <TitleBar />
        <div className="app-body">
          <Sidebar activeView={view} onViewChange={setView} version={appInfo.version} />
          <main className="content">
            {view === "home" ? (
              <HomeDashboard
                loaded={loaded}
                needsSetup={needsSetup}
                settings={settings}
                onSettingsChange={setSettings}
                saveState={saveState}
                recordingStatus={recordingStatus}
                statusMessage={statusMessage}
                lastResult={lastResult}
                latency={latency}
                stats={history.stats}
                latestEntry={history.entries[0] ?? null}
                onToggleRecording={toggleRecording}
                onOpenSettings={() => setView("settings")}
              />
            ) : null}
            {view === "history" ? <HistoryView history={history} onHistoryChange={setHistory} /> : null}
            {view === "settings" ? (
              <SettingsView
                settings={settings}
                defaultSystemPrompt={appInfo.defaultSystemPrompt}
                saveState={saveState}
                onChange={setSettings}
              />
            ) : null}
          </main>
        </div>
      </div>
    </I18nProvider>
  );
}

function initialView(): View {
  const hash = window.location.hash.slice(1);
  return hash === "history" || hash === "settings" ? hash : "home";
}

export default App;
