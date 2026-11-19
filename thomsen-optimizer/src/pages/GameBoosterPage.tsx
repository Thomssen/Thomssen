import { useEffect, useState } from "react";
import { Pencil, Plus, Power, RotateCcw, Trash2 } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Toggle } from "@/components/ui/Toggle";
import { Modal, ConfirmDialog } from "@/components/ui/Modal";
import { Field, TextInput, Textarea, Select } from "@/components/ui/Field";
import { SettingRow, Divider } from "@/components/ui/SettingRow";
import { useToast } from "@/components/ui/Toast";
import { useMonitor } from "@/state/MonitorProvider";
import { useLoadedSettings } from "@/state/SettingsProvider";
import { useAsync } from "@/hooks/useAsync";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { DEFAULT_PROFILES, POWER_LABELS } from "@/config/profiles";
import { formatDateTime } from "@/lib/format";
import type { PowerChoice, ProfileConfig } from "@/types";
import "./GameBoosterPage.css";

export function GameBoosterPage() {
  const { settings, update } = useLoadedSettings();
  const { booster, refreshBooster, lastEvent } = useMonitor();
  const toast = useToast();
  const [editing, setEditing] = useState<ProfileConfig | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<ProfileConfig | null>(null);
  const procs = useAsync(() => api.processes(), []);

  const save = async (patch: Parameters<typeof update>[0]) => {
    const err = await update(patch);
    if (err) toast.error(err);
    return !err;
  };

  const activate = async (p: ProfileConfig, closeApps: boolean) => {
    setBusy(p.id);
    try {
      const r = await api.activateProfile(p.id, closeApps);
      toast.success(`${p.name} activated${r.failed ? ` (${r.failed} item failed)` : ""}.`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
      refreshBooster();
    }
  };

  const deactivate = async () => {
    setBusy("restore");
    try {
      const ev = await api.deactivateProfile();
      if (ev.type === "restored") toast.success(ev.failed.length ? `Restored ${ev.restored}; ${ev.failed.length} failed.` : `Restored ${ev.restored} change${ev.restored === 1 ? "" : "s"}.`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
      refreshBooster();
    }
  };

  const saveProfile = async (p: ProfileConfig) => {
    const profiles = isNew ? [...settings.profiles, p] : settings.profiles.map((x) => (x.id === p.id ? p : x));
    if (await save({ profiles })) setEditing(null);
  };

  const remove = async (p: ProfileConfig) => {
    const profiles = settings.profiles.filter((x) => x.id !== p.id);
    const defaultProfileId = settings.defaultProfileId === p.id ? "balanced" : settings.defaultProfileId;
    await save({ profiles, defaultProfileId });
    setConfirmDelete(null);
  };

  return (
    <Page
      title="Game Booster"
      eyebrow="Gaming"
      description="Gaming profiles apply a small set of reversible Windows settings while a game runs, then restore them when the last game closes. Profiles make no FPS promises - measure the result on the Performance page."
      actions={
        <Button variant="secondary" icon={<Plus size={15} />} onClick={() => { setIsNew(true); setEditing({ ...DEFAULT_PROFILES[0]!, id: `custom-${Date.now().toString(36)}`, name: "New profile", builtin: false, description: "", tradeoffs: "" }); }}>
          New profile
        </Button>
      }
    >
      <Section>
        <Card className={booster?.active ? "gb__status gb__status--on" : "gb__status"}>
          <div className="gb__status-main">
            <div>
              <div className="gb__status-title">
                <Power size={16} /> {booster?.active ? `${booster.profileName} is active` : "Gaming mode is off"}
              </div>
              <p className="gb__status-sub">
                {booster?.active
                  ? `${booster.manual ? "Started manually" : "Started by a detected game"}${booster.startedAt ? ` · ${formatDateTime(booster.startedAt)}` : ""} · ${booster.changeCount} Windows setting${booster.changeCount === 1 ? "" : "s"} changed${booster.games.length ? ` · ${booster.games.join(", ")}` : ""}`
                  : "Nothing is changed until a game with auto-boost starts, or you activate a profile below."}
              </p>
            </div>
            {booster?.active && (
              <Button variant="secondary" icon={<RotateCcw size={15} />} onClick={deactivate} loading={busy === "restore"}>
                Restore now
              </Button>
            )}
          </div>
          {lastEvent && (
            <p className="gb__event">
              {lastEvent.event.type === "activated" && `Last activation: ${lastEvent.event.game} → ${lastEvent.event.profileName} (${lastEvent.event.outcomes.filter((o) => o.outcome === "applied").length} change(s) applied)`}
              {lastEvent.event.type === "failed" && `Could not activate for ${lastEvent.event.game}: ${lastEvent.event.message}`}
              {lastEvent.event.type === "restored" && `Last restore: ${lastEvent.event.restored} change(s) restored${lastEvent.event.failed.length ? `, ${lastEvent.event.failed.length} failed` : ""}`}
            </p>
          )}
        </Card>
      </Section>

      <Section title="Profiles" description="Edit any profile to match how you play. The default profile is used for games without their own.">
        <div className="gb__profiles">
          {settings.profiles.map((p) => (
            <Card key={p.id} className="gb__profile">
              <div className="gb__profile-head">
                <h3>{p.name}</h3>
                {settings.defaultProfileId === p.id ? <Badge tone="good">Default</Badge> : <button type="button" className="gb__link" onClick={() => save({ defaultProfileId: p.id })}>Make default</button>}
              </div>
              <p className="gb__profile-desc">{p.description || "Custom profile."}</p>
              <ul className="gb__facts">
                <li>Power plan: <strong>{POWER_LABELS[p.power]}</strong></li>
                <li>Game Mode: <strong>{p.gameMode ? "Ensure on" : "Unchanged"}</strong></li>
                <li>Game Bar recording: <strong>{p.disableGameDvr ? "Turned off" : "Unchanged"}</strong></li>
                <li>Apps to close: <strong>{p.closeApps.length ? p.closeApps.join(", ") : "None"}</strong></li>
              </ul>
              {p.tradeoffs && <p className="gb__tradeoff">Trade-offs: {p.tradeoffs}</p>}
              <div className="gb__profile-actions">
                <Button size="sm" variant="primary" onClick={() => activate(p, false)} loading={busy === p.id} disabled={!!booster?.active}>
                  Activate
                </Button>
                {p.closeApps.length > 0 && (
                  <Button size="sm" variant="secondary" onClick={() => activate(p, true)} disabled={!!booster?.active || busy !== null}>
                    Activate and close apps
                  </Button>
                )}
                <Button size="sm" variant="ghost" icon={<Pencil size={14} />} onClick={() => { setIsNew(false); setEditing(p); }}>
                  Edit
                </Button>
                {!p.builtin && (
                  <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} onClick={() => setConfirmDelete(p)} aria-label="Delete profile" />
                )}
              </div>
            </Card>
          ))}
        </div>
      </Section>

      <Section title="Automatic activation">
        <Card>
          <SettingRow title="Detect games automatically" hint="Checks the running programs every few seconds (less often while a game runs) against your game library.">
            <Toggle checked={settings.autoDetectGames} onChange={(v) => save({ autoDetectGames: v })} label="Detect games automatically" />
          </SettingRow>
          <Divider />
          <SettingRow title="Enable Game Booster automatically" hint="When a game that has auto-boost turned on starts, its profile is activated. Auto-boost is set per game on the Games page.">
            <Toggle checked={settings.autoBoost} onChange={(v) => save({ autoBoost: v })} label="Enable Game Booster automatically" />
          </SettingRow>
          <Divider />
          <SettingRow title="Restore settings when games close" hint="Puts every changed setting back as soon as the last boosted game has closed. If the app is closed or crashes during a game, leftovers are restored at the next start.">
            <Toggle checked={settings.restoreWhenGameCloses} onChange={(v) => save({ restoreWhenGameCloses: v })} label="Restore settings when games close" />
          </SettingRow>
          <p className="set__note">Automatic activation never closes applications - only the manual "Activate and close apps" button does, and only for apps you listed in the profile. Several games at once share one session; settings are restored when all of them have closed.</p>
        </Card>
      </Section>

      <ProfileEditor
        profile={editing}
        isNew={isNew}
        runningNames={(procs.data ?? []).filter((g) => !g.protected).map((g) => g.name)}
        onClose={() => setEditing(null)}
        onSave={saveProfile}
      />
      <ConfirmDialog open={!!confirmDelete} onClose={() => setConfirmDelete(null)} onConfirm={() => confirmDelete && remove(confirmDelete)} danger title="Delete this profile?" confirmLabel="Delete" message={`Games using "${confirmDelete?.name}" fall back to the default profile.`} />
    </Page>
  );
}

function ProfileEditor({ profile, isNew, runningNames, onClose, onSave }: { profile: ProfileConfig | null; isNew: boolean; runningNames: string[]; onClose: () => void; onSave: (p: ProfileConfig) => void }) {
  const [draft, setDraft] = useState<ProfileConfig | null>(profile);
  const [exe, setExe] = useState("");
  useEffect(() => setDraft(profile), [profile]);
  if (!draft) return null;
  const set = (patch: Partial<ProfileConfig>) => setDraft({ ...draft, ...patch });
  const exeOk = /^[^\\/:*?"<>|]{1,92}\.exe$/i.test(exe.trim());
  const factory = DEFAULT_PROFILES.find((d) => d.id === draft.id);

  return (
    <Modal
      open={!!profile}
      onClose={onClose}
      title={isNew ? "New profile" : `Edit ${draft.name}`}
      width={560}
      footer={
        <>
          {factory && (
            <Button variant="ghost" onClick={() => setDraft(factory)}>
              Reset to default
            </Button>
          )}
          <span style={{ flex: 1 }} />
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" disabled={!draft.name.trim()} onClick={() => onSave({ ...draft, name: draft.name.trim() })}>
            Save
          </Button>
        </>
      }
    >
      <div className="stack">
        <Field label="Name">
          <TextInput value={draft.name} maxLength={40} onChange={(e) => set({ name: e.target.value })} />
        </Field>
        <Field label="Description">
          <Textarea rows={2} value={draft.description} maxLength={400} onChange={(e) => set({ description: e.target.value })} />
        </Field>
        <Field label="Windows power plan while gaming" hint="Higher plans keep the CPU at higher clocks: possibly steadier frame times, but more power use and heat.">
          <Select
            value={draft.power}
            onChange={(v) => set({ power: v as PowerChoice })}
            options={(Object.keys(POWER_LABELS) as PowerChoice[]).map((k) => ({ value: k, label: POWER_LABELS[k]! }))}
            aria-label="Power plan"
          />
        </Field>
        <SettingRow title="Make sure Windows Game Mode is on">
          <Toggle checked={draft.gameMode} onChange={(v) => set({ gameMode: v })} label="Game Mode" />
        </SettingRow>
        <SettingRow title="Turn off Game Bar background recording" hint="Stops continuous background capture while the profile is active.">
          <Toggle checked={draft.disableGameDvr} onChange={(v) => set({ disableGameDvr: v })} label="Game Bar recording" />
        </SettingRow>
        <Field label="Apps to close on manual activation" hint="Only used by the manual 'Activate and close apps' button. Protected Windows, security and anti-cheat processes are never closed, whatever is listed here.">
          <div className="gb__chips">
            {draft.closeApps.map((a) => (
              <button key={a} type="button" className="gb__chip" onClick={() => set({ closeApps: draft.closeApps.filter((x) => x !== a) })} title="Remove">
                {a} ×
              </button>
            ))}
            {draft.closeApps.length === 0 && <span className="gb__none">None</span>}
          </div>
          <div className="gb__add">
            <TextInput value={exe} placeholder="Program.exe" onChange={(e) => setExe(e.target.value)} list="gb-running" />
            <datalist id="gb-running">{runningNames.slice(0, 40).map((n) => <option key={n} value={n} />)}</datalist>
            <Button size="sm" variant="secondary" disabled={!exeOk || draft.closeApps.length >= 40} onClick={() => { set({ closeApps: [...new Set([...draft.closeApps, exe.trim()])] }); setExe(""); }}>
              Add
            </Button>
          </div>
        </Field>
        <Field label="Trade-offs shown to you">
          <Textarea rows={2} value={draft.tradeoffs} maxLength={400} onChange={(e) => set({ tradeoffs: e.target.value })} />
        </Field>
      </div>
    </Modal>
  );
}
