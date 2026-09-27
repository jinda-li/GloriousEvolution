import { invoke } from "@tauri-apps/api/core";
import { Check, Copy, Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";

import type { HistoryEntry, HistoryStore } from "../types";
import { formatTime } from "../utils/format";

type HistoryViewProps = {
  history: HistoryStore;
  onHistoryChange: (history: HistoryStore) => void;
};

export function HistoryView({ history, onHistoryChange }: HistoryViewProps) {
  const [query, setQuery] = useState("");
  const [confirmClear, setConfirmClear] = useState(false);

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
          <h1>历史记录</h1>
          <p className="muted">最近 500 条语音输入，只保存在本机。</p>
        </div>
        <div className="page-head-actions">
          <label className="search">
            <Search size={15} />
            <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索" />
          </label>
          <button
            type="button"
            className={`button ${confirmClear ? "danger" : "ghost"}`}
            disabled={history.entries.length === 0}
            onClick={() => void clearAll()}
          >
            <Trash2 size={15} />
            {confirmClear ? "再点一次确认清空" : "清空"}
          </button>
        </div>
      </div>

      {entries.length === 0 ? (
        <div className="card empty-state">
          {history.entries.length === 0 ? "还没有历史记录。" : "没有匹配的记录。"}
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

  async function copy() {
    await invoke("copy_text", { text: showRaw ? entry.rawText : entry.text });
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <li className="card history-item">
      <div className="history-meta">
        <span>{formatTime(entry.createdAt)}</span>
        <span>{Math.round(entry.durationSeconds)} 秒</span>
        {entry.polished && entry.rawText !== entry.text ? (
          <button type="button" className="link-button" onClick={() => setShowRaw((value) => !value)}>
            {showRaw ? "看润色结果" : "看识别原文"}
          </button>
        ) : null}
        <span className="spacer" />
        <button type="button" className="icon-button" aria-label="复制" title="复制" onClick={() => void copy()}>
          {copied ? <Check size={15} /> : <Copy size={15} />}
        </button>
        <button type="button" className="icon-button" aria-label="删除" title="删除" onClick={onDelete}>
          <Trash2 size={15} />
        </button>
      </div>
      <p className={showRaw ? "raw" : ""}>{showRaw ? entry.rawText : entry.text}</p>
    </li>
  );
}
