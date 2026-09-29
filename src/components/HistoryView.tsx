import { invoke } from "@tauri-apps/api/core";
import { Check, Copy, Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";

import { useT } from "../i18n";
import type { HistoryEntry, HistoryStore } from "../types";
import { formatTime } from "../utils/format";

type HistoryViewProps = {
  history: HistoryStore;
  onHistoryChange: (history: HistoryStore) => void;
};

export function HistoryView({ history, onHistoryChange }: HistoryViewProps) {
  const [query, setQuery] = useState("");
  const [confirmClear, setConfirmClear] = useState(false);
  const t = useT();

  const entries = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) {
      return history.entries;
    }
    return history.entries.filter(
      (entry) => entry.text.toLowerCase().includes(needle) || entry.rawText.toLowerCase().includes(needle),
    );
  }, [history.entries, query]);

  async function remove(id: string) {
    onHistoryChange(await invoke<HistoryStore>("delete_history_entry", { id }));
  }

  async function clearAll() {
    if (!confirmClear) {
      setConfirmClear(true);
      window.setTimeout(() => setConfirmClear(false), 3000);
      return;
    }
    onHistoryChange(await invoke<HistoryStore>("clear_history"));
    setConfirmClear(false);
  }

  return (
    <section className="page history">
      <div className="page-head">
        <div>
          <h1>{t("history.title")}</h1>
          <p className="muted">{t("history.subtitle")}</p>
        </div>
        <div className="page-head-actions">
          <label className="search">
            <Search size={15} />
            <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("history.search")} />
          </label>
          <button
            type="button"
            className={`button ${confirmClear ? "danger" : "ghost"}`}
            disabled={history.entries.length === 0}
            onClick={() => void clearAll()}
          >
            <Trash2 size={15} />
            {confirmClear ? t("history.confirmClear") : t("history.clear")}
          </button>
        </div>
      </div>

      {entries.length === 0 ? (
        <div className="card empty-state">
          {history.entries.length === 0 ? t("history.empty") : t("history.noMatch")}
        </div>
      ) : (
        <ul className="history-list">
          {entries.map((entry) => (
            <HistoryItem key={entry.id} entry={entry} onDelete={() => void remove(entry.id)} />
          ))}
        </ul>
      )}
    </section>
  );
}

function HistoryItem({ entry, onDelete }: { entry: HistoryEntry; onDelete: () => void }) {
  const [copied, setCopied] = useState(false);
  const [showRaw, setShowRaw] = useState(false);
  const t = useT();

  async function copy() {
    await invoke("copy_text", { text: showRaw ? entry.rawText : entry.text });
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <li className="card history-item">
      <div className="history-meta">
        <span>{formatTime(entry.createdAt, t)}</span>
        <span>{t("time.seconds", { s: Math.round(entry.durationSeconds) })}</span>
        {entry.polished && entry.rawText !== entry.text ? (
          <button type="button" className="link-button" onClick={() => setShowRaw((value) => !value)}>
            {showRaw ? t("history.showPolished") : t("history.showRaw")}
          </button>
        ) : null}
        <span className="spacer" />
        <button
          type="button"
          className="icon-button"
          aria-label={t("common.copy")}
          title={t("common.copy")}
          onClick={() => void copy()}
        >
          {copied ? <Check size={15} /> : <Copy size={15} />}
        </button>
        <button
          type="button"
          className="icon-button"
          aria-label={t("common.delete")}
          title={t("common.delete")}
          onClick={onDelete}
        >
          <Trash2 size={15} />
        </button>
      </div>
      <p className={showRaw ? "raw" : ""}>{showRaw ? entry.rawText : entry.text}</p>
    </li>
  );
}
