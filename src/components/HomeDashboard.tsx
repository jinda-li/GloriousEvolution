import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Check, Clock, Copy, Mic, Sparkles, Square, Timer, Type } from "lucide-react";
import { useState } from "react";
import type { ReactNode } from "react";

import type { SaveState } from "../App";
import { useT } from "../i18n";
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
  const t = useT();

  return (
    <section className="page onboarding">
      <div className="onboarding-head">
        <span className="eyebrow">{t("onboarding.eyebrow")}</span>
        <h1>{t("onboarding.title")}</h1>
        <p>{t("onboarding.intro")}</p>
      </div>

      <ol className="steps">
        <li>
          <span className="step-index">1</span>
          <div>
            <strong>{t("onboarding.step1.title")}</strong>
            <p>{t("onboarding.step1.body")}</p>
          </div>
        </li>
        <li>
          <span className="step-index">2</span>
          <div className="step-body">
            <strong>{t("onboarding.step2.title")}</strong>
            <ApiKeyField settings={settings} onChange={onChange} onVerified={() => setVerified(true)} autoFocus />
          </div>
        </li>
        <li>
          <span className="step-index">3</span>
          <div>
            <strong>{t.rich("onboarding.step3.title", { shortcut: <kbd>{settings.shortcut}</kbd> })}</strong>
            <p>
              {settings.recordMode === "hold" ? t("onboarding.step3.hold") : t("onboarding.step3.toggle")}{" "}
              {t("onboarding.step3.more")}
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
          {t("onboarding.start")}
        </button>
        {!verified ? <span className="muted">{t("onboarding.testFirst")}</span> : null}
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
  const t = useT();
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

  const headline = isRecording ? t("home.listening") : isProcessing ? t("home.processing") : t("home.ready");
  const hint = settings.recordMode === "hold" ? t("home.hint.hold") : t("home.hint.toggle");

  return (
    <section className="page home">
      <div className={`record-panel ${recordingStatus}`}>
        <button
          type="button"
          className="record-button"
          onClick={onToggleRecording}
          disabled={isProcessing}
          aria-label={isRecording ? t("home.stopRecording") : t("home.startRecording")}
        >
          {isRecording ? <Square size={26} /> : <Mic size={30} />}
        </button>
        <div className="record-copy">
          <h1>{headline}</h1>
          <p>{hint}</p>
          <div className="shortcut-chip">
            {t("home.shortcut")} <kbd>{settings.shortcut}</kbd>
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
        <Stat icon={<Clock size={16} />} value={formatDuration(stats.totalSeconds, t)} label={t("stats.spoken")} />
        <Stat icon={<Type size={16} />} value={stats.totalWords.toLocaleString(t.locale)} label={t("stats.words")} />
        <Stat
          icon={<Timer size={16} />}
          value={t("stats.minutes", { n: minutesSaved.toLocaleString(t.locale) })}
          label={t("stats.saved")}
        />
        <Stat
          icon={<Sparkles size={16} />}
          value={stats.totalSessions.toLocaleString(t.locale)}
          label={t("stats.sessions")}
        />
      </div>

      <div className="card last-result">
        <div className="card-head">
          <h2>{t("home.latest")}</h2>
          {latency ? (
            <span className="muted small">{t("home.took", { s: (latency.totalMs / 1000).toFixed(1) })}</span>
          ) : null}
          {latest ? (
            <button type="button" className="button ghost small" onClick={() => void copyLast()}>
              {copied ? <Check size={14} /> : <Copy size={14} />}
              {copied ? t("common.copied") : t("common.copy")}
            </button>
          ) : null}
        </div>
        {latest ? (
          <div className="result-grid">
            <article>
              <span className="label">{t("home.raw")}</span>
              <p>{latest.raw}</p>
            </article>
            <article className="accent">
              <span className="label">{t("home.final")}</span>
              <p>{latest.text}</p>
            </article>
          </div>
        ) : (
          <p className="empty">{t("home.empty", { shortcut: settings.shortcut })}</p>
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
