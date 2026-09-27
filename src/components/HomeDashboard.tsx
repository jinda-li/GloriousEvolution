import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Check, Clock, Copy, Mic, Sparkles, Square, Timer, Type } from "lucide-react";
import { useState } from "react";
import type { ReactNode } from "react";

import type { SaveState } from "../App";
import type {
  AppSettings,
  HistoryEntry,
  LatencyMetricsEvent,
  ProcessResult,
  RecordingStatus,
  UsageStats,
} from "../types";
import { estimateMinutesSaved, formatDuration } from "../utils/format";
import { ApiKeyField } from "./ApiKeyField";

type HomeDashboardProps = {
  loaded: boolean;
  needsSetup: boolean;
  settings: AppSettings;
  onSettingsChange: (settings: AppSettings) => void;
  saveState: SaveState;
  recordingStatus: RecordingStatus;
  statusMessage: string;
  lastResult: ProcessResult | null;
  latency: LatencyMetricsEvent | null;
  stats: UsageStats;
  latestEntry: HistoryEntry | null;
  onToggleRecording: () => void;
  onOpenSettings: () => void;
};

export function HomeDashboard(props: HomeDashboardProps) {
  if (!props.loaded) {
    return <section className="page" />;
  }
  if (props.needsSetup || !props.settings.onboarded) {
    return <Onboarding settings={props.settings} onChange={props.onSettingsChange} />;
  }
  return <Dashboard {...props} />;
}

function Onboarding({ settings, onChange }: { settings: AppSettings; onChange: (settings: AppSettings) => void }) {
  const [verified, setVerified] = useState(false);

  return (
    <section className="page onboarding">
      <div className="onboarding-head">
        <span className="eyebrow">欢迎使用</span>
        <h1>说话，文字就出现在光标处</h1>
        <p>
          在任何应用里按下快捷键开始说话，再按一次结束。GloriousEvolution 会识别语音、去掉口头禅和重复，润色后直接输入到当前输入框。
        </p>
      </div>

      <ol className="steps">
        <li>
          <span className="step-index">1</span>
          <div>
            <strong>准备 OpenRouter API Key</strong>
            <p>在 openrouter.ai 注册并充值少量余额（$5 通常够用几个月），创建一个 Key。</p>
          </div>
        </li>
        <li>
          <span className="step-index">2</span>
          <div className="step-body">
            <strong>粘贴 Key 并测试</strong>
            <ApiKeyField
              settings={settings}
              onChange={onChange}
              onVerified={() => setVerified(true)}
              autoFocus
            />
          </div>
        </li>
        <li>
          <span className="step-index">3</span>
          <div>
            <strong>
              试一试：按 <kbd>{settings.shortcut}</kbd> 开始说话
            </strong>
            <p>
              {settings.recordMode === "hold" ? "按住说话，松开结束。" : "再按一次结束，Esc 取消。"}
              快捷键和其它选项可在「设置」中修改。
            </p>
          </div>
        </li>
      </ol>

      <div className="onboarding-actions">
        <button
          type="button"
          className="button primary large"
          disabled={!settings.apiKey || !verified}
          onClick={() => onChange({ ...settings, onboarded: true })}
        >
          <Check size={17} />
          开始使用
        </button>
        {!verified ? <span className="muted">测试连接成功后即可开始</span> : null}
      </div>
    </section>
  );
}

function Dashboard({
  settings,
  recordingStatus,
  statusMessage,
  lastResult,
  latency,
  stats,
  latestEntry,
  onToggleRecording,
}: HomeDashboardProps) {
  const [copied, setCopied] = useState(false);
  const isRecording = recordingStatus === "recording";
  const isProcessing = recordingStatus === "processing";
  const isError = recordingStatus === "error";
  const minutesSaved = estimateMinutesSaved(stats.totalWords, stats.totalSeconds);
  const latest = lastResult
    ? { raw: lastResult.rawText, text: lastResult.optimizedText }
    : latestEntry
      ? { raw: latestEntry.rawText, text: latestEntry.text }
      : null;

  async function copyLast() {
    if (!latest) {
      return;
    }
    await invoke("copy_text", { text: latest.text });
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  }

  const headline = isRecording ? "正在聆听…" : isProcessing ? "正在处理…" : "准备就绪";
  const hint =
    settings.recordMode === "hold"
      ? "在任意应用中按住快捷键说话，松开后文字会输入到光标处。"
      : "在任意应用中按一次快捷键开始说话，再按一次结束，Esc 取消。";

  return (
    <section className="page home">
      <div className={`record-panel ${recordingStatus}`}>
        <button
          type="button"
          className="record-button"
          onClick={onToggleRecording}
          disabled={isProcessing}
          aria-label={isRecording ? "结束录音" : "开始录音"}
        >
          {isRecording ? <Square size={26} /> : <Mic size={30} />}
        </button>
        <div className="record-copy">
          <h1>{headline}</h1>
          <p>{hint}</p>
          <div className="shortcut-chip">
            快捷键 <kbd>{settings.shortcut}</kbd>
          </div>
        </div>
      </div>

      {isError && statusMessage ? (
        <div className="notice danger">
          <AlertTriangle size={16} />
          <span>{statusMessage}</span>
        </div>
      ) : null}
      {lastResult?.warning ? (
        <div className="notice warning">
          <AlertTriangle size={16} />
          <span>{lastResult.warning}</span>
        </div>
      ) : null}

      <div className="stats-row">
        <Stat icon={<Clock size={16} />} value={formatDuration(stats.totalSeconds)} label="累计口述" />
        <Stat icon={<Type size={16} />} value={stats.totalWords.toLocaleString()} label="累计字数" />
        <Stat icon={<Timer size={16} />} value={`${minutesSaved} 分钟`} label="估计节省打字时间" />
        <Stat icon={<Sparkles size={16} />} value={stats.totalSessions.toLocaleString()} label="语音输入次数" />
      </div>

      <div className="card last-result">
        <div className="card-head">
          <h2>最近一次</h2>
          {latency ? <span className="muted small">用时 {(latency.totalMs / 1000).toFixed(1)} 秒</span> : null}
          {latest ? (
            <button type="button" className="button ghost small" onClick={() => void copyLast()}>
              {copied ? <Check size={14} /> : <Copy size={14} />}
              {copied ? "已复制" : "复制"}
            </button>
          ) : null}
        </div>
        {latest ? (
          <div className="result-grid">
            <article>
              <span className="label">识别原文</span>
              <p>{latest.raw}</p>
            </article>
            <article className="accent">
              <span className="label">最终输出</span>
              <p>{latest.text}</p>
            </article>
          </div>
        ) : (
          <p className="empty">还没有记录。切换到任意输入框，按下 {settings.shortcut} 试试。</p>
        )}
      </div>
    </section>
  );
}

function Stat({ icon, value, label }: { icon: ReactNode; value: string; label: string }) {
  return (
    <div className="card stat">
      <span className="stat-icon">{icon}</span>
      <strong>{value}</strong>
      <span className="muted small">{label}</span>
    </div>
  );
}
