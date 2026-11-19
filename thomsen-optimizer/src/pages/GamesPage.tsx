import { useEffect, useState } from "react";
import { Plus, RefreshCw } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Toggle } from "@/components/ui/Toggle";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { Modal } from "@/components/ui/Modal";
import { Field, TextInput } from "@/components/ui/Field";
import { Menu } from "@/components/ui/Menu";
import { useToast } from "@/components/ui/Toast";
import { ProfilePicker } from "@/components/ProfilePicker";
import { useLoadedSettings } from "@/state/SettingsProvider";
import { useAsync } from "@/hooks/useAsync";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { relativeTime } from "@/lib/format";
import { formatBytes1024 } from "@/lib/units";
import type { Game, GameSettings, Launcher } from "@/types";
import "./GamesPage.css";

const LAUNCHER: Record<Launcher, string> = { steam: "Steam", epic: "Epic Games", xbox: "Xbox app", battleNet: "Battle.net", custom: "Custom" };

export function GamesPage() {
  const { settings } = useLoadedSettings();
  const toast = useToast();
  const { data, loading, error, reload, setData } = useAsync(() => api.games(false), []);
  const [scanning, setScanning] = useState(false);
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");
  const [exe, setExe] = useState("");

  // Running state refreshes while this page is open; cheap (process list only).
  useEffect(() => {
    const t = window.setInterval(() => void reload(), 4000);
    return () => window.clearInterval(t);
  }, [reload]);

  const rescan = async () => {
    setScanning(true);
    try {
      setData(await api.games(true));
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setScanning(false);
    }
  };

  const patch = async (g: Game, change: Partial<GameSettings>) => {
    const next = { ...g.settings, ...change };
    setData((data ?? []).map((x) => (x.id === g.id ? { ...x, settings: next } : x)));
    try {
      await api.updateGame(g.id, next);
    } catch (e) {
      toast.error(errorMessage(e));
      void reload();
    }
  };

  const browse = async () => {
    const p = await api.pickExecutable("Choose the game's executable");
    if (p) {
      setExe(p);
      if (!name) setName(p.split(/[\\/]/).pop()!.replace(/\.exe$/i, ""));
    }
  };

  const add = async () => {
    try {
      await api.addGame(name.trim(), exe);
      setAdding(false);
      setName("");
      setExe("");
      await reload();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const gpuPref = async (g: Game, high: boolean) => {
    try {
      const out = await api.setGpuPreference(g.id, high);
      toast.success(out[0]?.outcome === "alreadySet" ? "Already set." : `Graphics preference set for ${g.name}. Restore it from History if needed.`);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const games = data ?? [];
  return (
    <Page
      title="Games"
      eyebrow="Library"
      description="Installed games found from Steam, Epic Games, the Xbox app and Battle.net, plus any program you add yourself. Each game remembers its own profile and options."
      actions={
        <>
          <Button variant="secondary" icon={<RefreshCw size={15} />} onClick={rescan} loading={scanning}>
            Rescan
          </Button>
          <Button variant="primary" icon={<Plus size={15} />} onClick={() => setAdding(true)}>
            Add game
          </Button>
        </>
      }
    >
      <Section>
        {loading && !data ? (
          <LoadingRow label="Looking for installed games" />
        ) : error ? (
          <EmptyState title="Could not read the game library" description={error} />
        ) : games.length === 0 ? (
          <EmptyState title="No games found" description="No supported launcher installs were detected. The Xbox app and Battle.net are best-effort; use Add game for anything missing." action={<Button onClick={() => setAdding(true)}>Add game</Button>} />
        ) : (
          <Card flush>
            <div className="table-wrap">
              <table className="table games__table">
                <thead>
                  <tr>
                    <th>Game</th>
                    <th>Launcher</th>
                    <th>Profile</th>
                    <th>Auto boost</th>
                    <th>Manage apps</th>
                    <th>Memory</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {games.map((g) => (
                    <tr key={g.id}>
                      <td>
                        <div className="table__primary games__name">
                          {g.name}
                          {g.running && <Badge tone="good">Running</Badge>}
                        </div>
                        <div className="table__mono games__path" title={g.exePath ?? g.installDir ?? ""}>
                          {g.exePath ?? g.installDir ?? "Executable unknown"}
                        </div>
                        <div className="table__muted games__sub">{g.lastPlayed ? `Last played ${relativeTime(g.lastPlayed)}` : "Last played: not available"}</div>
                      </td>
                      <td>{LAUNCHER[g.launcher]}</td>
                      <td style={{ minWidth: 190 }}>
                        <ProfilePicker profiles={settings.profiles} value={g.settings.profileId} onChange={(id) => patch(g, { profileId: id })} allowDefault defaultName={settings.profiles.find((p) => p.id === settings.defaultProfileId)?.name} />
                      </td>
                      <td>
                        <Toggle checked={g.settings.autoBoost} onChange={(v) => patch(g, { autoBoost: v })} label={`Auto boost for ${g.name}`} />
                      </td>
                      <td>
                        <Toggle checked={g.settings.backgroundManagement} onChange={(v) => patch(g, { backgroundManagement: v })} label={`Background app management for ${g.name}`} />
                      </td>
                      <td>
                        {g.settings.memoryMonitoring ? (
                          <span className="games__mem">{g.running ? formatBytes1024(g.memoryBytes) : "Not running"}</span>
                        ) : (
                          <span className="table__muted">Off</span>
                        )}
                        <Toggle checked={g.settings.memoryMonitoring} onChange={(v) => patch(g, { memoryMonitoring: v })} label={`Monitor memory for ${g.name}`} className="games__mem-toggle" />
                      </td>
                      <td className="table__actions">
                        <Menu
                          items={[
                            { label: "Use dedicated GPU (Windows graphics preference)", onSelect: () => gpuPref(g, true), disabled: !g.exePath },
                            { label: "Let Windows decide GPU", onSelect: () => gpuPref(g, false), disabled: !g.exePath },
                            g.launcher === "custom"
                              ? { label: "Remove from library", danger: true, onSelect: async () => { await api.removeGame(g.id).catch((e) => toast.error(errorMessage(e))); void reload(); } }
                              : { label: "Hide from library", onSelect: () => patch(g, { hidden: true }) },
                          ]}
                        />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        )}
        <p className="games__note">
          Games are recognised by the folder their program runs from, as Task Manager would see it. Thomsen Optimizer never reads game memory or injects anything, so it works alongside anti-cheat systems. Background app management uses the apps listed in the game's profile.
        </p>
      </Section>

      <Modal
        open={adding}
        onClose={() => setAdding(false)}
        title="Add a game"
        description="Pick the game's .exe file. It is only used to recognise the running game."
        footer={
          <>
            <Button variant="ghost" onClick={() => setAdding(false)}>
              Cancel
            </Button>
            <Button variant="primary" disabled={!name.trim() || !exe} onClick={add}>
              Add
            </Button>
          </>
        }
      >
        <div className="stack">
          <Field label="Executable">
            <div className="games__pick">
              <TextInput value={exe} readOnly placeholder="No file chosen" />
              <Button variant="secondary" onClick={browse}>
                Browse…
              </Button>
            </div>
          </Field>
          <Field label="Name">
            <TextInput value={name} maxLength={80} onChange={(e) => setName(e.target.value)} placeholder="Game name" />
          </Field>
        </div>
      </Modal>
    </Page>
  );
}
