import { useState } from "react";
import { Page } from "@/components/layout/Page";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { GeneralTab, GamingTab, MonitoringTab } from "./settings/BasicTabs";
import { OptimizationTab } from "./settings/OptimizationTab";
import { AppearanceTab } from "./settings/AppearanceTab";
import { AdvancedTab } from "./settings/AdvancedTab";
import { AboutTab } from "./settings/AboutTab";
import "./SettingsPage.css";

type Tab = "general" | "gaming" | "monitoring" | "optimization" | "appearance" | "advanced" | "about";
const TABS = [
  { value: "general", label: "General" },
  { value: "gaming", label: "Gaming" },
  { value: "monitoring", label: "Monitoring" },
  { value: "optimization", label: "Optimization" },
  { value: "appearance", label: "Appearance" },
  { value: "advanced", label: "Advanced" },
  { value: "about", label: "About" },
] as const;

export function SettingsPage() {
  const [tab, setTab] = useState<Tab>("general");
  return (
    <Page title="Settings" eyebrow="Preferences">
      <div className="stack">
        <SegmentedControl options={TABS} value={tab} onChange={setTab} aria-label="Settings sections" />
        <div key={tab} className="set__tab-panel">
          {tab === "general" && <GeneralTab />}
          {tab === "gaming" && <GamingTab />}
          {tab === "monitoring" && <MonitoringTab />}
          {tab === "optimization" && <OptimizationTab />}
          {tab === "appearance" && <AppearanceTab />}
          {tab === "advanced" && <AdvancedTab />}
          {tab === "about" && <AboutTab />}
        </div>
      </div>
    </Page>
  );
}
