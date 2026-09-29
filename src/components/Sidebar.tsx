import { History, Home, Settings } from "lucide-react";

import { useT } from "../i18n";
import type { MessageKey } from "../i18n";

export type View = "home" | "history" | "settings";

type SidebarProps = {
  activeView: View;
  version: string;
  onViewChange: (view: View) => void;
};

const navItems: Array<{ id: View; label: MessageKey; icon: typeof Home }> = [
  { id: "home", label: "nav.home", icon: Home },
  { id: "history", label: "nav.history", icon: History },
  { id: "settings", label: "nav.settings", icon: Settings },
];

export function Sidebar({ activeView, version, onViewChange }: SidebarProps) {
  const t = useT();
  return (
    <aside className="sidebar">
      <nav className="nav-list">
        {navItems.map((item) => {
          const Icon = item.icon;
          return (
            <button
              className={`nav-item ${activeView === item.id ? "active" : ""}`}
              key={item.id}
              onClick={() => onViewChange(item.id)}
              type="button"
            >
              <Icon size={17} />
              <span>{t(item.label)}</span>
            </button>
          );
        })}
      </nav>
      <div className="sidebar-footer">
        <span>{t("sidebar.poweredBy")}</span>
        {version ? <span>v{version}</span> : null}
      </div>
    </aside>
  );
}
