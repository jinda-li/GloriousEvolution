import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

import { useT } from "../i18n";
import { keyboardEventPreview, keyboardEventToShortcut } from "../utils/shortcut";

type ShortcutInputProps = {
  value: string;
  onChange: (shortcut: string) => void;
};

export function ShortcutInput({ value, onChange }: ShortcutInputProps) {
  const [listening, setListening] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const t = useT();

  useEffect(() => {
    if (!listening) {
      return;
    }

    void invoke("set_shortcut_capture_mode", { capturing: true });

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopImmediatePropagation();

      if (event.code === "Escape") {
        setListening(false);
        setPreview(null);
        return;
      }

      const nextPreview = keyboardEventPreview(event);
      if (nextPreview) {
        setPreview(nextPreview);
      }

      const shortcut = keyboardEventToShortcut(event);
      if (!shortcut) {
        return;
      }

      onChange(shortcut);
      setListening(false);
      setPreview(null);
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      void invoke("set_shortcut_capture_mode", { capturing: false });
      setPreview(null);
    };
  }, [listening, onChange]);

  useEffect(() => {
    if (!listening) {
      return;
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (buttonRef.current?.contains(event.target as Node)) {
        return;
      }

      setListening(false);
      setPreview(null);
    };

    window.addEventListener("pointerdown", handlePointerDown, true);
    return () => {
      window.removeEventListener("pointerdown", handlePointerDown, true);
    };
  }, [listening]);

  const displayValue = listening ? (preview ?? t("shortcut.press")) : value || t("shortcut.unset");

  return (
    <button
      ref={buttonRef}
      type="button"
      className={`shortcut-input${listening ? " listening" : ""}`}
      aria-pressed={listening}
      onClick={() => {
        setPreview(null);
        setListening(true);
      }}
    >
      <span>{displayValue}</span>
      {listening ? <span className="shortcut-input-hint">{t("shortcut.escCancel")}</span> : null}
    </button>
  );
}
