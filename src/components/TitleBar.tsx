import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, X } from "lucide-react";

import logo from "../assets/logo.svg";
import { useT } from "../i18n";

export function TitleBar() {
  const appWindow = getCurrentWindow();
  const t = useT();

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar-brand" data-tauri-drag-region>
        <img className="brand-mark" src={logo} alt="" aria-hidden="true" data-tauri-drag-region />
        <span data-tauri-drag-region>Sayso</span>
      </div>
      <div className="titlebar-actions">
        <button
          type="button"
          aria-label={t("titlebar.minimize")}
          title={t("titlebar.minimize")}
          onClick={() => void appWindow.minimize()}
        >
          <Minus size={15} />
        </button>
        <button
          type="button"
          className="close"
          aria-label={t("titlebar.close")}
          title={t("titlebar.closeHint")}
          onClick={() => void appWindow.close()}
        >
          <X size={15} />
        </button>
      </div>
    </header>
  );
}
