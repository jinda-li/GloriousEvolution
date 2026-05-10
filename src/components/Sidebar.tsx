import { Home, Settings } from "lucide-react";

type SidebarProps = {
  activeView: string;
  onViewChange: (view: string) => void;
};

const navItems = [
  { id: "home", label: "首页", icon: Home },
  { id: "settings", label: "设置", icon: Settings },
];

export function Sidebar({ activeView, onViewChange }: SidebarProps) {
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
              <Icon size={18} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
