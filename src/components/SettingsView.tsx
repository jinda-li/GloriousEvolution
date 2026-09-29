import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Check, Loader2, RefreshCw, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import type { ReactNode } from "react";

import type { SaveState } from "../App";
import { LANGUAGES, LLM_MODELS, OPENROUTER_BASE_URL, STT_MODELS } from "../constants";
import { LOCALES, useT } from "../i18n";
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
  const t = useT();
  const set = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => onChange({ ...settings, [key]: value });

  function refreshDevices() {
    void invoke<string[]>("list_input_devices").then(setDevices);
  }
  useEffect(refreshDevices, []);

  return (
    <section className="page settings">
      <div className="page-head">
        <div>
          <h1>{t("settings.title")}</h1>
          <p className="muted">{t("settings.autosave")}</p>
        </div>
        <SaveIndicator state={saveState} />
      </div>

      <Section title={t("settings.account")}>
        <ApiKeyField settings={settings} onChange={onChange} />
      </Section>

      <Section title={t("settings.shortcuts")}>
        <Row label={t("settings.shortcut")} hint={t("settings.shortcutHint")}>
          <ShortcutInput value={settings.shortcut} onChange={(shortcut) => set("shortcut", shortcut)} />
        </Row>
        <Row label={t("settings.mode")}>
          <Segmented<RecordMode>
            value={settings.recordMode}
            options={[
              { id: "toggle", label: t("settings.mode.toggle") },
              { id: "hold", label: t("settings.mode.hold") },
            ]}
            onChange={(mode) => set("recordMode", mode)}
          />
        </Row>
      </Section>

      <Section title={t("settings.speech")}>
        <Row label={t("settings.sttModel")}>
          <ModelPicker value={settings.sttModel} options={STT_MODELS} onChange={(id) => set("sttModel", id)} />
        </Row>
        <Row label={t("settings.spokenLanguage")} hint={t("settings.spokenLanguageHint")}>
          <select value={settings.language} onChange={(event) => set("language", event.target.value)}>
            {LANGUAGES.map((language) => (
              <option key={language.id} value={language.id}>
                {language.id === "auto" ? t("settings.autoDetect") : language.label}
              </option>
            ))}
          </select>
        </Row>
        <Row label={t("settings.mic")}>
          <div className="inline-controls">
            <select value={settings.inputDevice} onChange={(event) => set("inputDevice", event.target.value)}>
              <option value="">{t("settings.micDefault")}</option>
              {devices.map((device) => (
                <option key={device} value={device}>
                  {device}
                </option>
              ))}
              {settings.inputDevice && !devices.includes(settings.inputDevice) ? (
                <option value={settings.inputDevice}>
                  {t("settings.micDisconnected", { name: settings.inputDevice })}
                </option>
              ) : null}
            </select>
            <button
              type="button"
              className="icon-button"
              aria-label={t("settings.refreshDevices")}
              title={t("settings.refreshDevices")}
              onClick={refreshDevices}
            >
              <RefreshCw size={15} />
            </button>
          </div>
        </Row>
      </Section>

      <Section title={t("settings.polish")}>
        <Row label={t("settings.polishEnabled")} hint={t("settings.polishHint")}>
          <Toggle checked={settings.polishEnabled} onChange={(value) => set("polishEnabled", value)} />
        </Row>
        {settings.polishEnabled ? (
          <>
            <Row label={t("settings.llmModel")}>
              <ModelPicker value={settings.llmModel} options={LLM_MODELS} onChange={(id) => set("llmModel", id)} />
            </Row>
            <div className="field">
              <span className="field-label">{t("settings.dictionary")}</span>
              <textarea
                rows={3}
                value={settings.dictionary}
                placeholder={t("settings.dictionaryPlaceholder")}
                onChange={(event) => set("dictionary", event.target.value)}
              />
              <p className="field-hint">{t("settings.dictionaryHint")}</p>
            </div>
            <div className="field">
              <div className="field-label-row">
                <span className="field-label">{t("settings.prompt")}</span>
                <button
                  type="button"
                  className="link-button"
                  disabled={settings.systemPrompt === defaultSystemPrompt}
                  onClick={() => set("systemPrompt", defaultSystemPrompt)}
                >
                  <RotateCcw size={13} /> {t("settings.restoreDefault")}
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

      <Section title={t("settings.general")}>
        <Row label={t("settings.interfaceLanguage")}>
          <select value={settings.uiLanguage} onChange={(event) => set("uiLanguage", event.target.value)}>
            <option value="auto">{t("settings.followSystem")}</option>
            {LOCALES.map((locale) => (
              <option key={locale.id} value={locale.id}>
                {locale.label}
              </option>
            ))}
          </select>
        </Row>
        <Row label={t("settings.autoPaste")} hint={t("settings.autoPasteHint")}>
          <Toggle checked={settings.autoPaste} onChange={(value) => set("autoPaste", value)} />
        </Row>
        <Row label={t("settings.sound")}>
          <Toggle checked={settings.soundEnabled} onChange={(value) => set("soundEnabled", value)} />
        </Row>
        <Row label={t("settings.history")}>
          <Toggle checked={settings.historyEnabled} onChange={(value) => set("historyEnabled", value)} />
        </Row>
        <Row label={t("settings.launchAtLogin")} hint={t("settings.launchAtLoginHint")}>
          <Toggle checked={settings.launchAtLogin} onChange={(value) => set("launchAtLogin", value)} />
        </Row>
      </Section>

      <button type="button" className="link-button advanced-toggle" onClick={() => setShowAdvanced((v) => !v)}>
        {showAdvanced ? t("settings.hideAdvanced") : t("settings.showAdvanced")}
      </button>
      {showAdvanced ? (
        <Section title={t("settings.advanced")}>
          <div className="field">
            <div className="field-label-row">
              <span className="field-label">API Base URL</span>
              <button
                type="button"
                className="link-button"
                disabled={settings.baseUrl === OPENROUTER_BASE_URL}
                onClick={() => set("baseUrl", OPENROUTER_BASE_URL)}
              >
                <RotateCcw size={13} /> {t("settings.restoreOpenRouter")}
              </button>
            </div>
            <input
              value={settings.baseUrl}
              spellCheck={false}
              onChange={(event) => set("baseUrl", event.target.value)}
            />
            <p className="field-hint">{t("settings.baseUrlHint")}</p>
          </div>
        </Section>
      ) : null}
    </section>
  );
}

function SaveIndicator({ state }: { state: SaveState }) {
  const t = useT();
  if (state.kind === "saving") {
    return (
      <span className="save-indicator">
        <Loader2 className="spin" size={14} /> {t("settings.saving")}
      </span>
    );
  }
  if (state.kind === "saved") {
    return (
      <span className="save-indicator ok">
        <Check size={14} /> {t("settings.saved")}
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
  const t = useT();

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
        <option value={CUSTOM}>{t("settings.customModel")}</option>
      </select>
      {custom ? (
        <input
          value={value}
          spellCheck={false}
          placeholder="provider/model-name"
          onChange={(event) => onChange(event.target.value.trim())}
        />
      ) : (
        <span className="field-hint">{selected ? t(selected.note) : null}</span>
      )}
    </div>
  );
}
