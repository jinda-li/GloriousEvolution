import { invoke } from "@tauri-apps/api/core";
import { Eye, EyeOff, ExternalLink, Loader2, PlugZap } from "lucide-react";
import { useState } from "react";

import { OPENROUTER_CREDITS_URL, OPENROUTER_KEYS_URL } from "../constants";
import type { AppSettings, KeyStatus } from "../types";
import { formatUsd } from "../utils/format";

type ApiKeyFieldProps = {
  settings: AppSettings;
  onChange: (settings: AppSettings) => void;
  onVerified?: (status: KeyStatus) => void;
  autoFocus?: boolean;
};

type TestState =
  | { kind: "idle" }
  | { kind: "testing" }
  | { kind: "ok"; status: KeyStatus }
  | { kind: "error"; message: string };

export function openExternal(url: string) {
  void invoke("open_external", { url });
}

export function ApiKeyField({ settings, onChange, onVerified, autoFocus }: ApiKeyFieldProps) {
  const [visible, setVisible] = useState(false);
  const [test, setTest] = useState<TestState>({ kind: "idle" });

  async function runTest() {
    setTest({ kind: "testing" });
    try {
      const status = await invoke<KeyStatus>("test_connection", { newSettings: settings });
      setTest({ kind: "ok", status });
      onVerified?.(status);
    } catch (error) {
      setTest({ kind: "error", message: String(error) });
    }
  }

  const balance =
    test.kind === "ok" ? test.status.creditsRemaining ?? test.status.limitRemaining : null;

  return (
    <div className="field">
      <div className="field-label-row">
        <span className="field-label">OpenRouter API Key</span>
        <button type="button" className="link-button" onClick={() => openExternal(OPENROUTER_KEYS_URL)}>
          获取 Key <ExternalLink size={13} />
        </button>
      </div>
      <div className="input-with-actions">
        <input
          type={visible ? "text" : "password"}
          value={settings.apiKey}
          autoFocus={autoFocus}
          spellCheck={false}
          autoComplete="off"
          placeholder="sk-or-v1-..."
          onChange={(event) => {
            setTest({ kind: "idle" });
            onChange({ ...settings, apiKey: event.target.value.trim() });
          }}
        />
        <button
          type="button"
          className="icon-button"
          aria-label={visible ? "隐藏" : "显示"}
          onClick={() => setVisible((value) => !value)}
        >
          {visible ? <EyeOff size={16} /> : <Eye size={16} />}
        </button>
        <button
          type="button"
          className="button secondary"
          disabled={!settings.apiKey || test.kind === "testing"}
          onClick={() => void runTest()}
        >
          {test.kind === "testing" ? <Loader2 className="spin" size={15} /> : <PlugZap size={15} />}
          测试连接
        </button>
      </div>
      {test.kind === "ok" ? (
        <p className="field-hint success">
          连接成功{test.status.label ? `（${test.status.label}）` : ""}
          {balance !== null ? ` · 余额 ${formatUsd(balance)}` : ""}
        </p>
      ) : null}
      {test.kind === "ok" && test.status.creditsRemaining !== null && test.status.creditsRemaining < 0.5 ? (
        <p className="field-hint danger">
          余额低于 $0.50：OpenRouter 要求至少 $0.50 余额才能处理语音，充值后即可使用。
          <button type="button" className="link-button inline" onClick={() => openExternal(OPENROUTER_CREDITS_URL)}>
            去充值
          </button>
        </p>
      ) : null}
      {test.kind === "error" ? <p className="field-hint danger">{test.message}</p> : null}
      {test.kind === "idle" ? (
        <p className="field-hint">一个 Key 同时用于语音识别和文本润色。Key 只保存在本机。</p>
      ) : null}
    </div>
  );
}
