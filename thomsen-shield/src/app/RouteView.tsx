import { useNavigation } from "@/state/NavigationProvider";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { DashboardPage } from "@/pages/DashboardPage";
import { LiveConnectionsPage } from "@/pages/connections/LiveConnectionsPage";
import { ProtectionPage } from "@/pages/ProtectionPage";
import { FirewallPage } from "@/pages/firewall/FirewallPage";
import { BlocklistPage } from "@/pages/BlocklistPage";
import { AllowlistPage } from "@/pages/AllowlistPage";
import { EventsLogsPage } from "@/pages/logs/EventsLogsPage";
import { SettingsPage } from "@/pages/SettingsPage";

export function RouteView() {
  const { route } = useNavigation();

  return (
    <ErrorBoundary resetKey={route}>
      <div className="routeview" key={route}>
        {route === "dashboard" && <DashboardPage />}
        {route === "connections" && <LiveConnectionsPage />}
        {route === "protection" && <ProtectionPage />}
        {route === "firewall" && <FirewallPage />}
        {route === "blocklist" && <BlocklistPage />}
        {route === "allowlist" && <AllowlistPage />}
        {route === "logs" && <EventsLogsPage />}
        {route === "settings" && <SettingsPage />}
      </div>
    </ErrorBoundary>
  );
}
