import { useEffect } from "react";
import { Card, CardHeader } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { useToast } from "@/components/ui/Toast";
import { useAsync } from "@/hooks/useAsync";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";

const Facts = ({ evidence, limits, downsides }: { evidence: string; limits: string; downsides: string }) => (
  <dl className="adv__facts">
    <dt>Evidence</dt>
    <dd>{evidence}</dd>
    <dt>Limitations</dt>
    <dd>{limits}</dd>
    <dt>Downsides</dt>
    <dd>{downsides}</dd>
  </dl>
);

const AVOIDED: [string, string][] = [
  ["HPET / BCD timer changes", "Measured results vary widely between systems and some games get worse; not safe to apply blindly."],
  ["Network registry and TCP \"latency\" tweaks", "Modern Windows tunes TCP itself; popular tweaks are unverified and can break connectivity."],
  ["Disabling Windows services in bulk", "Breaks features (updates, audio, Store, Xbox, security) in ways that are hard to trace."],
  ["Disabling Windows Defender, core isolation or other security features", "Lowers protection for no reliable FPS benefit."],
  ["GPU driver registry edits", "Unverified, driver-specific and a common cause of crashes."],
  ["Forced process priority / CPU affinity", "Can starve the game's own threads or Windows; can trip anti-cheat heuristics."],
  ["\"RAM cleaning\" and registry cleaners", "Windows already uses free memory as cache; forcing it empty makes things slower. Registry cleaners offer no performance gain and add risk."],
  ["Kernel drivers, injection, hooking games", "Thomsen Optimizer installs no drivers and never touches game processes, so it cannot interfere with anti-cheat."],
];

/** The Advanced section: only options with a technical justification, each with evidence, limits and downsides. */
export function AdvancedTab() {
  const toast = useToast();
  const plans = useAsync(() => api.powerPlans(), []);
  useEffect(() => void plans.reload(), []); // eslint-disable-line react-hooks/exhaustive-deps

  const choose = async (guid: string, name: string) => {
    try {
      await api.setPowerPlan(guid);
      toast.success(`Power plan set to ${name}. Restore it from the history if you change your mind.`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      await plans.reload();
    }
  };

  return (
    <>
      <Card>
        <CardHeader title="Windows power plan" description="Applies immediately and is recorded in the restore history." />
        <div className="adv__item">
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 14 }}>
            {(plans.data ?? []).map((p) => (
              <Button key={p.guid} size="sm" variant={p.active ? "primary" : "secondary"} disabled={p.active || p.kind === "custom"} onClick={() => choose(p.guid, p.name)}>
                {p.name}
                {p.active ? " (active)" : ""}
              </Button>
            ))}
            {plans.error && <span className="table__muted">{plans.error}</span>}
          </div>
          <Facts
            evidence="Power plans set CPU boost behaviour and minimum clocks. On laptops and some desktops, 'Power saver' demonstrably lowers game performance; the Balanced plan is close to High performance on most modern CPUs."
            limits="Gains over Balanced are usually small and CPU-dependent. Custom OEM plans are managed in Windows."
            downsides="Higher plans increase power use, heat and fan noise, and reduce laptop battery life."
          />
        </div>
      </Card>

      <Card>
        <CardHeader title="Game Mode & Game Bar" />
        <div className="adv__item">
          <h4>Windows Game Mode</h4>
          <Facts evidence="Introduced to prioritise the foreground game and defer driver/update activity." limits="Independent tests show small and inconsistent gains; a few older titles stutter with it." downsides="None serious - toggle it off if a game misbehaves." />
          <div style={{ marginTop: 12 }}>
            <Button size="sm" variant="secondary" onClick={() => api.openExternal("gameMode")}>Open Game Mode settings</Button>
          </div>
        </div>
        <div className="adv__item">
          <h4>Graphics preference per game</h4>
          <Facts evidence="Windows' per-app 'High performance' preference makes laptops with two GPUs use the dedicated one." limits="Only matters on systems with an integrated and a dedicated GPU. Set per game on the Games page." downsides="Higher power use on laptops." />
          <div style={{ marginTop: 12 }}>
            <Button size="sm" variant="secondary" onClick={() => api.openExternal("graphics")}>Open Windows graphics settings</Button>
          </div>
        </div>
        <div className="adv__item">
          <h4>Hardware-accelerated GPU scheduling <Badge dot={false}>Guidance only</Badge></h4>
          <Facts evidence="A Windows option (Settings > System > Display > Graphics) that moves GPU scheduling onto the GPU." limits="Mixed results: it helps some games (and frame generation) and hurts others. Needs a restart." downsides="Possible stutter on some drivers. This app does not change it - test it yourself with the Performance page." />
        </div>
        <div className="adv__item">
          <h4>Per-game profiles and process monitoring</h4>
          <p className="table__muted" style={{ fontSize: 12.5 }}>Configured on the Games and Game Booster pages. Games are recognised from the process list only; nothing is attached to or injected into them.</p>
        </div>
      </Card>

      <Card>
        <CardHeader title="What Thomsen Optimizer deliberately does not do" description="These popular 'tweaks' are not offered, because the evidence for them is weak or the risk is real." />
        <ul className="adv__avoid">
          {AVOIDED.map(([t, why]) => (
            <li key={t}>
              <strong>{t}</strong>
              <br />
              <span>{why}</span>
            </li>
          ))}
        </ul>
      </Card>
    </>
  );
}
