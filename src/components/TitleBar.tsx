import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, X } from "lucide-react";

export function TitleBar() {
  const appWindow = getCurrentWindow();

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar-brand" data-tauri-drag-region>
        <span className="brand-mark" aria-hidden="true" />
        <span data-tauri-drag-region>GloriousEvolution</span>
      </div>
      <div className="titlebar-actions">
        <button type="button" aria-label="最小化" title="最小化" onClick={() => void appWindow.minimize()}>
          <Minus size={15} />
        </button>
        <button
          type="button"
          className="close"
          aria-label="关闭到托盘"
          title="关闭到托盘（快捷键仍可用）"
          onClick={() => void appWindow.close()}
        >
          <X size={15} />
        </button>
      </div>
    </header>
  );
}
