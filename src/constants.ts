import type { AppSettings } from "./types";

export const OPENROUTER_BASE_URL = "https://openrouter.ai/api/v1";
export const OPENROUTER_KEYS_URL = "https://openrouter.ai/settings/keys";
export const OPENROUTER_CREDITS_URL = "https://openrouter.ai/settings/credits";

export type ModelOption = {
  id: string;
  label: string;
  note: string;
};

export const STT_MODELS: ModelOption[] = [
  { id: "openai/gpt-4o-mini-transcribe", label: "GPT-4o mini Transcribe", note: "推荐 · 准确，约 $0.003/分钟" },
  { id: "qwen/qwen3-asr-flash-2026-02-10", label: "Qwen3 ASR Flash", note: "中文与方言更强，约 $0.002/分钟" },
  { id: "openai/gpt-4o-transcribe", label: "GPT-4o Transcribe", note: "最高精度，约 $0.006/分钟" },
  { id: "openai/whisper-large-v3-turbo", label: "Whisper Large v3 Turbo", note: "最快最便宜，约 $0.0002/分钟" },
  { id: "google/gemini-3.5-transcribe", label: "Gemini 3.5 Transcribe", note: "多语种混说" },
  { id: "mistralai/voxtral-mini-transcribe", label: "Voxtral Mini Transcribe", note: "欧洲语言，约 $0.003/分钟" },
  { id: "deepgram/nova-3", label: "Deepgram Nova 3", note: "英文场景，约 $0.004/分钟" },
];

export const LLM_MODELS: ModelOption[] = [
  { id: "google/gemini-3.1-flash-lite", label: "Gemini 3.1 Flash Lite", note: "推荐 · 约 0.5 秒，每次不到 $0.001" },
  { id: "google/gemini-2.5-flash-lite", label: "Gemini 2.5 Flash Lite", note: "最便宜，速度快" },
  { id: "qwen/qwen3.7-flash", label: "Qwen 3.7 Flash", note: "中英混说稳，极便宜，稍慢" },
  { id: "openai/gpt-4.1-mini", label: "GPT-4.1 mini", note: "稳定可靠" },
  { id: "anthropic/claude-haiku-4.5", label: "Claude Haiku 4.5", note: "润色质量高，稍贵" },
];

export const LANGUAGES = [
  { id: "auto", label: "自动识别" },
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
};
