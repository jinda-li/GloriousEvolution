import { Clock, Keyboard, Mic, Square, Type } from "lucide-react";

type HomeDashboardProps = {
  shortcut: string;
  statusMessage: string;
  lastRawText: string;
  lastOptimizedText: string;
  recordingStatus: string;
  totalSeconds: number;
  totalWords: number;
  missingConfig: string;
  onToggleRecording: () => void;
  onOpenSettings: () => void;
};

export function HomeDashboard({
  shortcut,
  statusMessage,
  lastRawText,
  lastOptimizedText,
  recordingStatus,
  totalSeconds,
  totalWords,
  missingConfig,
  onToggleRecording,
  onOpenSettings,
}: HomeDashboardProps) {
  const isRecording = recordingStatus === "recording";
  const isProcessing = recordingStatus === "processing";
  const isMissingConfig = Boolean(missingConfig);

  return (
    <main className="content home-content">
      <section className="hero" data-tauri-drag-region>
        <div data-tauri-drag-region>
          <h1>GloriousEvolution</h1>
          <p>
            {isMissingConfig ? "开始使用前需要先填写 API Key。" : "按快捷键开始或停止语音输入，也可以使用下方按钮。"}
            {!isMissingConfig ? <kbd>{shortcut}</kbd> : null}
          </p>
        </div>
        <div className="hero-controls" aria-hidden="true" data-tauri-drag-region>
          <span />
          <span />
          <span />
        </div>
      </section>

      <section className="current-flow">
        <article className="record-card">
          <div className="record-icon">
            {isRecording ? <Square size={34} /> : <Mic size={38} />}
          </div>
          <div>
            <h2>{isMissingConfig ? "请先填写 API Key" : isProcessing ? "正在处理语音" : isRecording ? "正在录音" : "准备语音输入"}</h2>
            <p>{missingConfig || statusMessage}</p>
          </div>
          <button type="button" onClick={isMissingConfig ? onOpenSettings : onToggleRecording} disabled={isProcessing}>
            {isMissingConfig ? "去设置填写" : isRecording ? "结束录音" : isProcessing ? "处理中..." : "开始录音"}
          </button>
        </article>

        <article className="real-stat-card">
          <Clock size={19} />
          <strong>{formatDuration(totalSeconds)}</strong>
          <span>本次运行总口述时间</span>
        </article>

        <article className="real-stat-card">
          <Type size={19} />
          <strong>{totalWords}</strong>
          <span>本次运行输出字数</span>
        </article>

        <article className="real-stat-card">
          <Keyboard size={19} />
          <strong>{shortcut}</strong>
          <span>全局快捷键</span>
        </article>
      </section>

      <section className="last-result">
        <h2>最近一次识别</h2>
        {lastRawText || lastOptimizedText ? (
          <div className="result-grid">
            <article>
              <span>原始文本</span>
              <p>{lastRawText}</p>
            </article>
            <article>
              <span>优化后</span>
              <p>{lastOptimizedText}</p>
            </article>
          </div>
        ) : (
          <p className="empty-result">还没有语音输入记录。按快捷键开始第一段录音。</p>
        )}
      </section>
    </main>
  );
}

function formatDuration(seconds: number) {
  if (seconds < 60) {
    return `${seconds}s`;
  }

  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  if (minutes < 60) {
    return `${minutes}m ${rest}s`;
  }

  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}
