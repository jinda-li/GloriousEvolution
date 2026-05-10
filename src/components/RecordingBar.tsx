import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, Loader2, X } from "lucide-react";
import { useState } from "react";
import type { MouseEvent } from "react";

import type { RecordingStatus } from "../types";

type RecordingBarProps = {
  status: RecordingStatus;
  message: string;
  audioLevel?: number;
  progress?: number;
  previewText?: string;
  embedded?: boolean;
  onCancel?: () => void;
  onFinish?: () => void;
};

export function RecordingBar({
  status,
  message,
  audioLevel = 0,
  progress = 0,
  previewText = "",
  embedded = false,
  onCancel,
  onFinish,
}: RecordingBarProps) {
  const isProcessing = status === "processing";
  const [localError, setLocalError] = useState("");
  const displayMessage = localError || message;
  const hasPreview = Boolean(previewText);
  const isError = status === "error" || Boolean(localError);
  const statusText = getStatusText(status, displayMessage, isError);
  const dotFactors = [0.35, 0.55, 0.78, 1, 0.86, 0.64, 0.48, 0.38];

  async function cancel() {
    if (hasPreview) {
      onCancel?.();
      return;
    }

    try {
      await invoke("cancel_recording");
    } finally {
      onCancel?.();
    }
  }

  async function finish() {
    if (isError) {
      onFinish?.();
      return;
    }

    if (hasPreview) {
      try {
        await invoke("copy_text", { text: previewText });
      } finally {
        onFinish?.();
      }
      return;
    }

    try {
      setLocalError("");
      await invoke("stop_and_process");
      onFinish?.();
    } catch (error) {
      setLocalError(String(error));
    }
  }

  async function dragWindow(event: MouseEvent<HTMLElement>) {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button")) {
      return;
    }

    await getCurrentWindow().startDragging();
  }

  return (
    <div className={embedded ? "recording-shell embedded" : "recording-shell"} onMouseDown={dragWindow}>
      <div className={`recording-bar ${isError ? "error" : ""}`} title={displayMessage} data-tauri-drag-region>
        <button className="round-button cancel" type="button" onClick={cancel} disabled={isProcessing}>
          <X size={24} />
        </button>
        <div className="recording-status" aria-label={displayMessage}>
          {hasPreview ? (
            <div className="recording-preview">
              <span className="recording-message">预览</span>
              <span className="recording-preview-text">{previewText}</span>
            </div>
          ) : isError || isProcessing ? (
            <div className="recording-progress">
              <span className="recording-message">
                {isProcessing ? <Loader2 className="spin" size={14} /> : null}
                {statusText}
              </span>
              {isProcessing ? (
                <div className="recording-progress-track">
                  <span style={{ width: `${Math.round(progress * 100)}%` }} />
                </div>
              ) : null}
            </div>
          ) : (
            <div className="recording-dots">
              {dotFactors.map((factor, index) => {
                const size = 4 + audioLevel * 11 * factor;
                return (
                  <span
                    key={index}
                    style={{
                      width: `${size}px`,
                      height: `${size}px`,
                      opacity: 0.55 + audioLevel * 0.45,
                    }}
                  />
                );
              })}
            </div>
          )}
        </div>
        <button className="round-button confirm" type="button" onClick={finish} disabled={isProcessing}>
          {hasPreview ? <span className="copy-label">复制</span> : <Check size={25} />}
        </button>
      </div>
    </div>
  );
}

function getStatusText(status: RecordingStatus, message: string, isError: boolean) {
  if (isError) {
    if (message.includes("API Key") || message.includes("配置")) {
      return "配置错误";
    }
    if (message.includes("请求") || message.includes("401") || message.includes("Unauthorized")) {
      return "请求失败";
    }
    return "出错";
  }

  if (status === "processing") {
    return message && message.length <= 6 ? message : "处理中";
  }

  if (status === "done") {
    return "完成";
  }

  return "录音中";
}
