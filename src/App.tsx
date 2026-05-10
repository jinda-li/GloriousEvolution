import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo, useState } from "react";
import type { MouseEvent } from "react";

import { HomeDashboard } from "./components/HomeDashboard";
import { RecordingBar } from "./components/RecordingBar";
import { Sidebar } from "./components/Sidebar";
import type {
  AppSettings,
  AudioLevelEvent,
  LatencyMetricsEvent,
  ProcessResult,
  ProcessingProgressEvent,
  RecordingEvent,
  RecordingStatus,
  TextPreviewEvent,
} from "./types";

const defaultSettings: AppSettings = {
  elevenlabsApiKey: "",
  optimizerApiKey: "",
  optimizerBaseUrl: "https://api.openai.com/v1",
  optimizerModel: "gpt-4.1-mini",
  shortcut: "Alt+Q",
  autoPaste: true,
  systemPrompt:
    "你是一个语音输入文本优化助手。只处理人的语音内容：请删除背景音、拟声词、音效描述、音乐/掌声/噪声说明和明显误识别的非目标语言内容。请仅对文本进行语言表达优化，保持原意不变，修正语病、标点和口语化停顿，优化措辞和结构，让文本自然、清晰、适合直接发送。不得添加、扩展或推断原文未包含的内容；不得解释文本、回答文本中的问题或输出分析过程。最终只输出优化后的文本。如果没有有效的人声文本，输出空字符串。",
};

function App() {
  const [view, setView] = useState("home");
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus>("idle");
  const [statusMessage, setStatusMessage] = useState("准备就绪");
  const [lastResult, setLastResult] = useState<ProcessResult>({
    rawText: "",
    optimizedText: "",
    durationSeconds: 0,
    delivery: "inserted",
  });
  const [totalSeconds, setTotalSeconds] = useState(0);
  const [totalWords, setTotalWords] = useState(0);
  const [saveState, setSaveState] = useState("已保存");
  const [audioLevel, setAudioLevel] = useState(0);
  const [processingProgress, setProcessingProgress] = useState(0);
  const [previewText, setPreviewText] = useState("");

  const isRecorderWindow = useMemo(() => window.location.pathname === "/recorder", []);
  const missingConfig = getMissingConfig(settings);

  useEffect(() => {
    document.documentElement.classList.toggle("recorder-window", isRecorderWindow);
    document.body.classList.toggle("recorder-window", isRecorderWindow);

    invoke<AppSettings>("load_settings")
      .then(setSettings)
      .catch((error) => setStatusMessage(String(error)));

    const unlistenStatus = listen<RecordingEvent>("recording-status", (event) => {
      setRecordingStatus(event.payload.status);
      setStatusMessage(event.payload.message);
    });

    const unlistenResult = listen<ProcessResult>("process-result", (event) => applyProcessResult(event.payload));
    const unlistenAudioLevel = listen<AudioLevelEvent>("audio-level", (event) => {
      setAudioLevel(Math.max(0, Math.min(1, event.payload.level)));
    });
    const unlistenProgress = listen<ProcessingProgressEvent>("processing-progress", (event) => {
      setProcessingProgress(Math.max(0, Math.min(1, event.payload.progress)));
      setStatusMessage(event.payload.message);
    });
    const unlistenLatency = listen<LatencyMetricsEvent>("latency-metrics", (event) => {
      setStatusMessage(
        `完成，主要耗时：${event.payload.bottleneck}（总 ${Math.round(event.payload.totalMs / 1000)}s）`,
      );
    });
    const unlistenPreview = listen<TextPreviewEvent>("text-preview", (event) => {
      setPreviewText(event.payload.text);
    });
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.altKey && event.code === "KeyQ" && !event.repeat) {
        event.preventDefault();
        void toggleRecording();
      }
    };
    if (!isRecorderWindow) {
      window.addEventListener("keydown", handleKeyDown);
    }

    return () => {
      void unlistenStatus.then((dispose) => dispose());
      void unlistenResult.then((dispose) => dispose());
      void unlistenAudioLevel.then((dispose) => dispose());
      void unlistenProgress.then((dispose) => dispose());
      void unlistenLatency.then((dispose) => dispose());
      void unlistenPreview.then((dispose) => dispose());
      window.removeEventListener("keydown", handleKeyDown);
      document.documentElement.classList.remove("recorder-window");
      document.body.classList.remove("recorder-window");
    };
  }, [isRecorderWindow]);

  function applyProcessResult(result: ProcessResult) {
    setLastResult(result);
    setTotalSeconds((current) => current + Math.round(result.durationSeconds));
    setTotalWords((current) => current + countWords(result.optimizedText || result.rawText));
  }

  async function saveSettings() {
    setSaveState("保存中...");
    try {
      await saveCurrentSettings();
      setSaveState("已保存");
    } catch (error) {
      setSaveState(String(error));
    }
  }

  async function saveCurrentSettings() {
    await invoke("save_settings", { newSettings: settings });
  }

  async function toggleRecording() {
    if (missingConfig && recordingStatus !== "recording") {
      setStatusMessage(missingConfig);
      setView("settings");
      return;
    }

    try {
      if (recordingStatus === "recording") {
        await saveCurrentSettings();
        await stopAndCaptureResult();
      } else if (recordingStatus !== "processing") {
        await saveCurrentSettings();
        setSaveState("已保存");
        setPreviewText("");
        await invoke("toggle_recording");
      }
    } catch (error) {
      setRecordingStatus("error");
      setStatusMessage(String(error));
    }
  }

  async function stopAndCaptureResult() {
    try {
      await saveCurrentSettings();
      const result = await invoke<ProcessResult>("stop_and_process");
      setLastResult(result);
    } catch (error) {
      setRecordingStatus("error");
      setStatusMessage(String(error));
    }
  }

  async function handleWindowDrag(event: MouseEvent<HTMLElement>) {
    if (event.button !== 0) {
      return;
    }

    if (isInteractiveDragTarget(event.target)) {
      return;
    }

    await getCurrentWindow().startDragging();
  }

  if (isRecorderWindow) {
    return (
      <RecordingBar
        status={recordingStatus === "idle" ? "recording" : recordingStatus}
        message={statusMessage}
        audioLevel={audioLevel}
        progress={processingProgress}
        previewText={previewText}
        onCancel={() => {
          setPreviewText("");
          void getCurrentWindow().hide();
        }}
        onFinish={() => {
          setPreviewText("");
          void getCurrentWindow().hide();
        }}
      />
    );
  }

  return (
    <div className="app-shell" onMouseDown={handleWindowDrag}>
      <div className="background-drag-region" data-tauri-drag-region />
      <div className="window-drag-region" data-tauri-drag-region />
      <Sidebar activeView={view} onViewChange={setView} />
      {view === "home" ? (
        <HomeDashboard
          shortcut={settings.shortcut}
          statusMessage={statusMessage}
          lastRawText={lastResult.rawText}
          lastOptimizedText={lastResult.optimizedText}
          recordingStatus={recordingStatus}
          totalSeconds={totalSeconds}
          totalWords={totalWords}
          missingConfig={missingConfig}
          onToggleRecording={toggleRecording}
          onOpenSettings={() => setView("settings")}
        />
      ) : (
        <SettingsPanel
          settings={settings}
          saveState={saveState}
          onChange={setSettings}
          onSave={saveSettings}
          view={view}
        />
      )}
    </div>
  );
}

type SettingsPanelProps = {
  settings: AppSettings;
  saveState: string;
  view: string;
  onChange: (settings: AppSettings) => void;
  onSave: () => void;
};

function SettingsPanel({ settings, saveState, view, onChange, onSave }: SettingsPanelProps) {
  const missingConfig = getMissingConfig(settings);
  const provider = getProvider(settings.optimizerBaseUrl);
  const isBaseUrlLocked = provider !== "custom";

  function selectProvider(nextProvider: ModelProvider) {
    if (nextProvider === "openai") {
      onChange({
        ...settings,
        optimizerBaseUrl: "https://api.openai.com/v1",
        optimizerModel: settings.optimizerModel || "gpt-4.1-mini",
      });
      return;
    }

    if (nextProvider === "openrouter") {
      onChange({
        ...settings,
        optimizerBaseUrl: "https://openrouter.ai/api/v1",
        optimizerModel:
          provider === "openrouter" && settings.optimizerModel
            ? settings.optimizerModel
            : "openai/gpt-oss-120b:free",
      });
      return;
    }

    onChange({
      ...settings,
      optimizerBaseUrl: provider === "custom" ? settings.optimizerBaseUrl : "",
    });
  }

  if (view !== "settings") {
    return (
      <main className="content placeholder-view">
        <h1>设置</h1>
        <p>当前版本只保留已可用的语音输入和模型配置。</p>
      </main>
    );
  }

  return (
    <main className="content settings-view">
      <section className="settings-card">
        <div>
          <h1>设置</h1>
          <p>配置语音识别、文本优化模型和自动粘贴行为。</p>
        </div>

        {missingConfig ? <div className="settings-warning">{missingConfig}</div> : null}

        <label>
          ElevenLabs API Key
          <input
            type="password"
            value={settings.elevenlabsApiKey}
            onChange={(event) => onChange({ ...settings, elevenlabsApiKey: event.target.value })}
            placeholder="xi_..."
          />
        </label>

        <div className="provider-card">
          <div>
            <strong>文本模型 Provider</strong>
            <p>选择 OpenAI 或 OpenRouter 时会自动锁定 API Base URL；自定义时可手动填写。</p>
          </div>
          <div className="provider-options">
            {modelProviders.map((option) => (
              <label className="provider-option" key={option.id}>
                <input
                  type="radio"
                  name="model-provider"
                  checked={provider === option.id}
                  onChange={() => selectProvider(option.id)}
                />
                <span>{option.label}</span>
              </label>
            ))}
          </div>
        </div>

        <label>
          文本模型 API Key
          <input
            type="password"
            value={settings.optimizerApiKey}
            onChange={(event) => onChange({ ...settings, optimizerApiKey: event.target.value })}
            placeholder="OpenRouter: sk-or-v1-..."
          />
        </label>

        <div className="two-column">
          <label>
            API Base URL
            <input
              value={settings.optimizerBaseUrl}
              disabled={isBaseUrlLocked}
              onChange={(event) => onChange({ ...settings, optimizerBaseUrl: event.target.value })}
            />
          </label>
          <label>
            Model
            <input
              value={settings.optimizerModel}
              onChange={(event) => onChange({ ...settings, optimizerModel: event.target.value })}
            />
          </label>
        </div>

        <div className="two-column">
          <label>
            快捷键
            <input
              value={settings.shortcut}
              onChange={(event) => onChange({ ...settings, shortcut: event.target.value })}
            />
          </label>
          <label className="toggle-row">
            自动粘贴
            <input
              type="checkbox"
              checked={settings.autoPaste}
              onChange={(event) => onChange({ ...settings, autoPaste: event.target.checked })}
            />
          </label>
        </div>

        <label>
          优化提示词
          <textarea
            value={settings.systemPrompt}
            onChange={(event) => onChange({ ...settings, systemPrompt: event.target.value })}
            rows={6}
          />
        </label>

        <div className="settings-actions">
          <span>{saveState}</span>
          <button type="button" onClick={onSave}>
            保存设置
          </button>
        </div>
      </section>
    </main>
  );
}

export default App;

type ModelProvider = "openai" | "openrouter" | "custom";

const modelProviders: Array<{ id: ModelProvider; label: string }> = [
  { id: "openai", label: "OpenAI" },
  { id: "openrouter", label: "OpenRouter" },
  { id: "custom", label: "自定义" },
];

function countWords(text: string) {
  const chineseChars = text.match(/[\u4e00-\u9fff]/g)?.length ?? 0;
  const latinWords = text.match(/[A-Za-z0-9]+(?:['-][A-Za-z0-9]+)*/g)?.length ?? 0;
  return chineseChars + latinWords;
}

function getProvider(baseUrl: string): ModelProvider {
  const normalized = baseUrl.trim().replace(/\/+$/, "");
  if (normalized === "https://api.openai.com/v1") {
    return "openai";
  }

  if (normalized === "https://openrouter.ai/api/v1") {
    return "openrouter";
  }

  return "custom";
}

function getMissingConfig(settings: AppSettings) {
  if (!settings.elevenlabsApiKey.trim()) {
    return "需要填写 ElevenLabs API Key，否则无法识别语音。";
  }

  if (!settings.optimizerApiKey.trim()) {
    return "需要填写文本模型 API Key，否则无法优化文本。";
  }

  if (!settings.optimizerBaseUrl.trim()) {
    return "需要填写文本模型 API Base URL。";
  }

  if (!settings.optimizerModel.trim()) {
    return "需要填写文本模型名称。";
  }

  return "";
}

function isInteractiveDragTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return Boolean(
    target.closest(
      [
        "button",
        "input",
        "textarea",
        "select",
        "a",
        "label",
        "[role='button']",
        "[contenteditable='true']",
        ".provider-option",
      ].join(", "),
    ),
  );
}
