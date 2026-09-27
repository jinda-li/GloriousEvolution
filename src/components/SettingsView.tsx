import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Check, Loader2, RefreshCw, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import type { ReactNode } from "react";

import type { SaveState } from "../App";
import { LANGUAGES, LLM_MODELS, OPENROUTER_BASE_URL, STT_MODELS } from "../constants";
import type { ModelOption } from "../constants";
import type { AppSettings, RecordMode } from "../types";
import { ApiKeyField } from "./ApiKeyField";
import { ShortcutInput } from "./ShortcutInput";

type SettingsViewProps = {
  settings: AppSettings;
  defaultSystemPrompt: string;
  saveState: SaveState;
  onChange: (settings: AppSettings) => void;
};

export function SettingsView({ settings, defaultSystemPrompt, saveState, onChange }: SettingsViewProps) {
  const [devices, setDevices] = useState<string[]>([]);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const set = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) =>
    onChange({ ...settings, [key]: value });

  function refreshDevices() {
    void invoke<string[]>("list_input_devices").then(setDevices);
  }
  useEffect(refreshDevices, []);

  return (
    <section className="page settings">
      <div className="page-head">
        <div>
          <h1>设置</h1>
          <p className="muted">修改会自动保存。</p>
        </div>
        <SaveIndicator state={saveState} />
      </div>

      <Section title="账户">
        <ApiKeyField settings={settings} onChange={onChange} />
      </Section>

      <Section title="快捷键">
        <Row label="录音快捷键" hint="在任何应用里都可以使用。">
          <ShortcutInput value={settings.shortcut} onChange={(shortcut) => set("shortcut", shortcut)} />
        </Row>
        <Row label="触发方式">
          <Segmented<RecordMode>
            value={settings.recordMode}
            options={[
              { id: "toggle", label: "按一下开始 / 再按结束" },
              { id: "hold", label: "按住说话" },
            ]}
            onChange={(mode) => set("recordMode", mode)}
          />
        </Row>
      </Section>

      <Section title="语音识别">
        <Row label="识别模型">
          <ModelPicker value={settings.sttModel} options={STT_MODELS} onChange={(id) => set("sttModel", id)} />
        </Row>
        <Row label="说话语言" hint="固定语言可以提升准确率和速度。">
          <select value={settings.language} onChange={(event) => set("language", event.target.value)}>
            {LANGUAGES.map((language) => (
              <option key={language.id} value={language.id}>
                {language.label}
              </option>
            ))}
          </select>
        </Row>
        <Row label="麦克风">
          <div className="inline-controls">
            <select value={settings.inputDevice} onChange={(event) => set("inputDevice", event.target.value)}>
              <option value="">系统默认</option>
              {devices.map((device) => (
                <option key={device} value={device}>
                  {device}
                </option>
              ))}
              {settings.inputDevice && !devices.includes(settings.inputDevice) ? (
                <option value={settings.inputDevice}>{settings.inputDevice}（未连接）</option>
              ) : null}
            </select>
            <button type="button" className="icon-button" aria-label="刷新设备" title="刷新设备" onClick={refreshDevices}>
              <RefreshCw size={15} />
            </button>
          </div>
        </Row>
      </Section>

      <Section title="智能润色">
        <Row label="启用润色" hint="去掉口头禅、重复和改口，修正标点与错别字。关闭后直接输出识别原文，速度更快。">
          <Toggle checked={settings.polishEnabled} onChange={(value) => set("polishEnabled", value)} />
        </Row>
        {settings.polishEnabled ? (
          <>
            <Row label="润色模型">
              <ModelPicker value={settings.llmModel} options={LLM_MODELS} onChange={(id) => set("llmModel", id)} />
            </Row>
            <div className="field">
              <span className="field-label">个人词典</span>
              <textarea
                rows={3}
                value={settings.dictionary}
                placeholder="每行一个，或用逗号分隔。例如：OpenRouter, Tauri, 张三丰"
                onChange={(event) => set("dictionary", event.target.value)}
              />
              <p className="field-hint">人名、产品名、专业术语。润色时会把发音相近的词纠正成这里的写法。</p>
            </div>
            <div className="field">
              <div className="field-label-row">
                <span className="field-label">润色指令</span>
                <button
                  type="button"
                  className="link-button"
                  disabled={settings.systemPrompt === defaultSystemPrompt}
                  onClick={() => set("systemPrompt", defaultSystemPrompt)}
                >
                  <RotateCcw size={13} /> 恢复默认
                </button>
              </div>
              <textarea
                rows={8}
                value={settings.systemPrompt}
                onChange={(event) => set("systemPrompt", event.target.value)}
              />
            </div>
          </>
        ) : null}
      </Section>

      <Section title="输出与通用">
        <Row label="自动输入到光标处" hint="关闭后结果只显示在浮窗里，点击复制。">
          <Toggle checked={settings.autoPaste} onChange={(value) => set("autoPaste", value)} />
        </Row>
        <Row label="提示音">
          <Toggle checked={settings.soundEnabled} onChange={(value) => set("soundEnabled", value)} />
        </Row>
        <Row label="保存历史记录">
          <Toggle checked={settings.historyEnabled} onChange={(value) => set("historyEnabled", value)} />
        </Row>
        <Row label="开机自动启动" hint="启动后在系统托盘待命。">
          <Toggle checked={settings.launchAtLogin} onChange={(value) => set("launchAtLogin", value)} />
        </Row>
      </Section>

      <button type="button" className="link-button advanced-toggle" onClick={() => setShowAdvanced((v) => !v)}>
        {showAdvanced ? "收起高级设置" : "高级设置"}
      </button>
      {showAdvanced ? (
        <Section title="高级">
          <div className="field">
            <div className="field-label-row">
              <span className="field-label">API Base URL</span>
              <button
                type="button"
                className="link-button"
                disabled={settings.baseUrl === OPENROUTER_BASE_URL}
                onClick={() => set("baseUrl", OPENROUTER_BASE_URL)}
              >
                <RotateCcw size={13} /> 恢复 OpenRouter
              </button>
            </div>
            <input
              value={settings.baseUrl}
              spellCheck={false}
              onChange={(event) => set("baseUrl", event.target.value)}
            />
            <p className="field-hint">
              默认使用 OpenRouter。也可以填写任何同时兼容 OpenAI /audio/transcriptions 与 /chat/completions 的接口。
            </p>
          </div>
        </Section>
      ) : null}
    </section>
  );
}

function SaveIndicator({ state }: { state: SaveState }) {
  if (state.kind === "saving") {
    return (
      <span className="save-indicator">
        <Loader2 className="spin" size={14} /> 保存中
      </span>
    );
  }
  if (state.kind === "saved") {
    return (
      <span className="save-indicator ok">
        <Check size={14} /> 已保存
      </span>
    );
  }
  if (state.kind === "error") {
    return (
      <span className="save-indicator error" title={state.message}>
        <AlertTriangle size={14} /> {state.message}
      </span>
    );
  }
  return null;
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="card settings-section">
      <h2>{title}</h2>
      {children}
    </div>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="setting-row">
      <div className="setting-copy">
        <span className="field-label">{label}</span>
        {hint ? <span className="field-hint">{hint}</span> : null}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (value: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className={`toggle ${checked ? "on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <span />
    </button>
  );
}

function Segmented<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: Array<{ id: T; label: string }>;
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmented" role="radiogroup">
      {options.map((option) => (
        <button
          key={option.id}
          type="button"
          role="radio"
          aria-checked={value === option.id}
          className={value === option.id ? "active" : ""}
          onClick={() => onChange(option.id)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

const CUSTOM = "__custom__";

function ModelPicker({
  value,
  options,
  onChange,
}: {
  value: string;
  options: ModelOption[];
  onChange: (id: string) => void;
}) {
  const isPreset = options.some((option) => option.id === value);
  const [custom, setCustom] = useState(!isPreset);
  const selected = options.find((option) => option.id === value);

  return (
    <div className="model-picker">
      <select
        value={custom ? CUSTOM : value}
        onChange={(event) => {
          if (event.target.value === CUSTOM) {
            setCustom(true);
            return;
          }
          setCustom(false);
          onChange(event.target.value);
        }}
      >
        {options.map((option) => (
          <option key={option.id} value={option.id}>
            {option.label}
          </option>
        ))}
        <option value={CUSTOM}>自定义模型…</option>
      </select>
      {custom ? (
        <input
          value={value}
          spellCheck={false}
          placeholder="provider/model-name"
          onChange={(event) => onChange(event.target.value.trim())}
        />
      ) : (
        <span className="field-hint">{selected?.note}</span>
      )}
    </div>
  );
}
