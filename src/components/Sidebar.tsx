import { History, Home, Settings } from "lucide-react";

export type View = "home" | "history" | "settings";

type SidebarProps = {
  activeView: View;
  version: string;
  onViewChange: (view: View) => void;
};

const navItems: Array<{ id: View; label: string; icon: typeof Home }> = [
  { id: "home", label: "首页", icon: Home },
  { id: "history", label: "历史记录", icon: History },
  { id: "settings", label: "设置", icon: Settings },
];

export function Sidebar({ activeView, version, onViewChange }: SidebarProps) {
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
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>
      <div className="sidebar-footer">
        <span>OpenRouter 驱动</span>
        {version ? <span>v{version}</span> : null}
      </div>
    </aside>
  );
}
