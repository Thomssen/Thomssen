import { lazy, Suspense } from "react";
import { useNavigation } from "@/state/NavigationProvider";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { DashboardPage } from "@/pages/DashboardPage";
import { OptimizePage } from "@/pages/OptimizePage";
import { GameBoosterPage } from "@/pages/GameBoosterPage";
import { GamesPage } from "@/pages/GamesPage";
import { LoadingRow } from "@/components/ui/Spinner";

// Charts (Recharts) are only loaded when the Performance page is opened.
const PerformancePage = lazy(() => import("@/pages/PerformancePage").then((m) => ({ default: m.PerformancePage })));
import { CleanerPage } from "@/pages/CleanerPage";
import { StartupPage } from "@/pages/StartupPage";
import { SystemInfoPage } from "@/pages/SystemInfoPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { ResultsPage } from "@/pages/ResultsPage";

export function RouteView() {
  const { route } = useNavigation();
  return (
    <ErrorBoundary resetKey={route}>
      <div className="routeview" key={route}>
        {route === "dashboard" && <DashboardPage />}
        {route === "optimize" && <OptimizePage />}
        {route === "game-booster" && <GameBoosterPage />}
        {route === "games" && <GamesPage />}
        {route === "performance" && (
          <Suspense fallback={<LoadingRow label="Loading charts" />}>
            <PerformancePage />
          </Suspense>
        )}
        {route === "cleaner" && <CleanerPage />}
        {route === "startup" && <StartupPage />}
        {route === "system" && <SystemInfoPage />}
        {route === "settings" && <SettingsPage />}
        {route === "results" && <ResultsPage />}
      </div>
    </ErrorBoundary>
  );
}
