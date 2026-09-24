import { APP } from "@/config/app";
import { NAV_GROUPS, SETTINGS_ITEM, type Route } from "@/config/navigation";
import { useNavigation } from "@/state/NavigationProvider";
import { Badge } from "@/components/ui/Badge";
import "./Sidebar.css";

interface SidebarProps {
  protectionActive: boolean;
}

export function Sidebar({ protectionActive }: SidebarProps) {
  const { route, navigate } = useNavigation();

  const isActive = (r: Route) => route === r;
  const linkClass = (r: Route) => ["sidebar__link", isActive(r) ? "is-active" : ""].filter(Boolean).join(" ");

  return (
    <aside className="sidebar">
      <div className="sidebar__brand">
        <span className="sidebar__mark" aria-hidden="true">
          S
        </span>
        <span className="sidebar__wordmark">{APP.brand}</span>
      </div>

      <nav className="sidebar__nav" aria-label="Primary">
        {NAV_GROUPS.map((group, gi) => (
          <div className="sidebar__group" key={gi}>
            {group.map((item) => (
              <button key={item.route} type="button" className={linkClass(item.route)} aria-current={isActive(item.route) ? "page" : undefined} onClick={() => navigate(item.route)}>
                {item.label}
              </button>
            ))}
          </div>
        ))}
      </nav>

      <div className="sidebar__footer">
        <button type="button" className={linkClass(SETTINGS_ITEM.route)} aria-current={isActive(SETTINGS_ITEM.route) ? "page" : undefined} onClick={() => navigate(SETTINGS_ITEM.route)}>
          {SETTINGS_ITEM.label}
        </button>
        <div className="sidebar__footer-right">
          <Badge tone={protectionActive ? "good" : "neutral"}>{protectionActive ? "Protected" : "Off"}</Badge>
        </div>
      </div>
    </aside>
  );
}
