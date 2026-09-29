import type { MessageKey } from "./i18n";
import type { AppSettings } from "./types";

export const OPENROUTER_BASE_URL = "https://openrouter.ai/api/v1";
export const OPENROUTER_KEYS_URL = "https://openrouter.ai/settings/keys";
export const OPENROUTER_CREDITS_URL = "https://openrouter.ai/settings/credits";

export type ModelOption = {
  id: string;
  label: string;
  note: MessageKey;
};

export const STT_MODELS: ModelOption[] = [
  { id: "openai/gpt-4o-mini-transcribe", label: "GPT-4o mini Transcribe", note: "model.stt.gpt4oMini" },
  { id: "qwen/qwen3-asr-flash-2026-02-10", label: "Qwen3 ASR Flash", note: "model.stt.qwen" },
  { id: "openai/gpt-4o-transcribe", label: "GPT-4o Transcribe", note: "model.stt.gpt4o" },
  { id: "openai/whisper-large-v3-turbo", label: "Whisper Large v3 Turbo", note: "model.stt.whisper" },
  { id: "google/gemini-3.5-transcribe", label: "Gemini 3.5 Transcribe", note: "model.stt.gemini" },
  { id: "mistralai/voxtral-mini-transcribe", label: "Voxtral Mini Transcribe", note: "model.stt.voxtral" },
  { id: "deepgram/nova-3", label: "Deepgram Nova 3", note: "model.stt.deepgram" },
];

export const LLM_MODELS: ModelOption[] = [
  { id: "google/gemini-3.1-flash-lite", label: "Gemini 3.1 Flash Lite", note: "model.llm.gemini31" },
  { id: "google/gemini-2.5-flash-lite", label: "Gemini 2.5 Flash Lite", note: "model.llm.gemini25" },
  { id: "qwen/qwen3.7-flash", label: "Qwen 3.7 Flash", note: "model.llm.qwen" },
  { id: "openai/gpt-4.1-mini", label: "GPT-4.1 mini", note: "model.llm.gpt41mini" },
  { id: "anthropic/claude-haiku-4.5", label: "Claude Haiku 4.5", note: "model.llm.haiku" },
];

// Spoken languages for recognition; "auto" is labelled by the UI locale.
export const LANGUAGES = [
  { id: "auto", label: "" },
  { id: "zh", label: "中文" },
  { id: "en", label: "English" },
  { id: "ja", label: "日本語" },
  { id: "ko", label: "한국어" },
  { id: "fr", label: "Français" },
  { id: "de", label: "Deutsch" },
  { id: "es", label: "Español" },
  { id: "ru", label: "Русский" },
];

export const DEFAULT_SETTINGS: AppSettings = {
  apiKey: "",
  baseUrl: OPENROUTER_BASE_URL,
  sttModel: STT_MODELS[0].id,
  llmModel: LLM_MODELS[0].id,
  polishEnabled: true,
  language: "auto",
  shortcut: "Alt+Q",
  recordMode: "toggle",
  autoPaste: true,
  inputDevice: "",
  dictionary: "",
  systemPrompt: "",
  soundEnabled: true,
  historyEnabled: true,
  launchAtLogin: false,
  onboarded: false,
  uiLanguage: "auto",
};
