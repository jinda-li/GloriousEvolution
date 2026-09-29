// Browser-only preview (npm run dev without Tauri): fakes the backend so pages render.
// Query parameters: ?lang=<locale> picks the UI language, ?onboarding shows first-run,
// and on /recorder, ?status=recording|processing|preview|done picks the pill state.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";

import { DEFAULT_SETTINGS } from "./constants";
import type { HistoryEntry } from "./types";

type Sample = { raw: string; text: string; seconds: number };

const SAMPLES: Record<"en" | "zh", Sample[]> = {
  en: [
    {
      raw: "um so can we move the design review to uh thursday at three no wait four",
      text: "Can we move the design review to Thursday at 4?",
      seconds: 5.4,
    },
    {
      raw: "note to self pick up the dry cleaning and uh call mom about the weekend",
      text: "Note to self: pick up the dry cleaning and call Mom about the weekend.",
      seconds: 4.8,
    },
    {
      raw: "the three things for the launch are the landing page the press kit and the uh the demo video",
      text: "The three things for the launch are:\n1. The landing page\n2. The press kit\n3. The demo video",
      seconds: 7.1,
    },
    {
      raw: "thanks so much for the quick turnaround this looks great ship it",
      text: "Thanks so much for the quick turnaround. This looks great, ship it!",
      seconds: 3.6,
    },
  ],
  zh: [
    {
      raw: "嗯那个我们明天下午三点开会呃不对是四点",
      text: "我们明天下午四点开会。",
      seconds: 5.2,
    },
    {
      raw: "帮我记一下就是周五之前要把那个 open router 的发票整理好",
      text: "记一下：周五之前要把 OpenRouter 的发票整理好。",
      seconds: 4.3,
    },
    {
      raw: "发布前要做三件事第一个是落地页第二个是那个宣传素材然后第三个是演示视频",
      text: "发布前要做三件事：\n1. 落地页\n2. 宣传素材\n3. 演示视频",
      seconds: 6.9,
    },
    {
      raw: "辛苦了这版改得很好就这么上线吧",
      text: "辛苦了，这版改得很好，就这么上线吧！",
      seconds: 3.1,
    },
  ],
};

export function installDevMock() {
  const params = new URLSearchParams(location.search);
  const isRecorder = location.pathname === "/recorder";
  const uiLanguage = params.get("lang") ?? "auto";
  const samples = SAMPLES[uiLanguage.startsWith("zh") ? "zh" : "en"];
  const now = Date.now();
  const entries: HistoryEntry[] = samples.map((sample, index) => ({
    id: String(index + 1),
    createdAt: now - [2, 45, 26 * 60, 50 * 60][index] * 60_000,
    rawText: sample.raw,
    text: sample.text,
    durationSeconds: sample.seconds,
    polished: true,
  }));

  mockWindows(isRecorder ? "recorder" : "main");
  mockIPC(
    (cmd) => {
      switch (cmd) {
        case "load_settings":
          return {
            ...DEFAULT_SETTINGS,
            apiKey: "sk-or-v1-demo",
            uiLanguage,
            onboarded: !params.has("onboarding"),
            systemPrompt: "…",
          };
        case "app_info":
          return { version: "0.3.0", defaultSystemPrompt: "…" };
        case "get_history":
          return { stats: { totalSeconds: 13_834, totalWords: 41_210, totalSessions: 1_142 }, entries };
        case "list_input_devices":
          return ["Microphone (Realtek High Definition Audio)", "AirPods Pro"];
        case "test_connection":
          return { label: "demo", usage: 0.1, limit: null, limitRemaining: null, isFreeTier: false, creditsRemaining: 4.2 };
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );

  if (isRecorder) {
    const status = params.get("status") ?? "recording";
    window.setTimeout(() => void playRecorder(status, samples[0].text), 50);
  }
}

async function playRecorder(status: string, text: string) {
  await emit("recording-status", { status: status === "preview" ? "processing" : status, message: "" });
  if (status === "recording") {
    let tick = 0;
    window.setInterval(() => {
      tick += 1;
      const level = Math.abs(Math.sin(tick * 0.9) * Math.cos(tick * 0.37)) * 0.9 + 0.05;
      void emit("audio-level", { level });
    }, 80);
  }
  if (status === "processing") {
    await emit("processing-progress", { phase: "optimize", message: "", progress: 0.65 });
  }
  if (status === "preview") {
    await emit("text-preview", { text });
    await emit("recording-status", { status: "preview", message: "" });
  }
}
