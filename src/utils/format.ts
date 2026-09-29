import type { Translate } from "../i18n";

export function formatDuration(totalSeconds: number, t: Translate) {
  const seconds = Math.round(totalSeconds);
  if (seconds < 60) {
    return t("time.seconds", { s: seconds });
  }
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return t("time.minutes", { m: minutes, s: seconds % 60 });
  }
  const hours = Math.floor(minutes / 60);
  return t("time.hours", { h: hours, m: minutes % 60 });
}

export function formatClock(totalSeconds: number) {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function formatTime(timestamp: number, t: Translate) {
  const date = new Date(timestamp);
  const now = new Date();
  const time = date.toLocaleTimeString(t.locale, { hour: "2-digit", minute: "2-digit" });
  if (date.toDateString() === now.toDateString()) {
    return t("time.today", { time });
  }
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (date.toDateString() === yesterday.toDateString()) {
    return t("time.yesterday", { time });
  }
  return `${date.toLocaleDateString(t.locale, { month: "short", day: "numeric" })} ${time}`;
}

export function formatUsd(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `$${value.toFixed(value < 1 ? 3 : 2)}`;
}

// Typing at ~40 characters per minute versus speaking; used for the "time saved" stat.
export function estimateMinutesSaved(words: number, spokenSeconds: number) {
  const typingMinutes = words / 40;
  return Math.max(0, Math.round(typingMinutes - spokenSeconds / 60));
}
