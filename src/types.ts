export type AppSettings = {
  elevenlabsApiKey: string;
  optimizerApiKey: string;
  optimizerBaseUrl: string;
  optimizerModel: string;
  shortcut: string;
  autoPaste: boolean;
  systemPrompt: string;
};

export type RecordingStatus = "idle" | "recording" | "processing" | "done" | "error" | "preview";

export type RecordingEvent = {
  status: RecordingStatus;
  message: string;
};

export type AudioLevelEvent = {
  level: number;
};

export type ProcessingProgressEvent = {
  phase: string;
  message: string;
  progress: number;
};

export type LatencyMetricsEvent = {
  totalMs: number;
  sttMs: number;
  filterMs: number;
  optimizeMs: number;
  pasteMs: number;
  bottleneck: string;
};

export type TextPreviewEvent = {
  text: string;
};

export type ProcessResult = {
  rawText: string;
  optimizedText: string;
  durationSeconds: number;
  delivery: "inserted" | "needsCopy";
};
