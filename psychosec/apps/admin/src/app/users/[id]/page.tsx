"use client";
import { useParams } from "next/navigation";
import { useState } from "react";
import { Ban, CheckCircle2, LogOut, ShieldOff, Undo2 } from "lucide-react";
import type { AdminAction } from "@psychosec/shared";
import { ConfirmDialog, type ConfirmSpec } from "@/components/confirm";
import { PageTitle, useAdmin } from "@/components/shell";
import { Avatar, Badge, Button, Card, CardHeader, ErrorNote, Loading, Table, Td } from "@/components/ui";
import { api, fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";

interface Detail {
  user: { id: string; discord_id: string; username: string; display_name: string | null; avatar_url: string | null; access_status: string; guild_member: boolean; role_verified: boolean; created_at: string; last_login_at: string | null; last_activity_at: string | null };
  is_admin: boolean;
  sessions: { id: string; kind: string; created_at: string; last_seen_at: string; expires_at: string; revoked_at: string | null; application_version: string | null; ip: string | null }[];
  logins: { id: number; ip: string | null; ip_country: string | null; result: string; created_at: string; application_version: string | null; client: string }[];
  bans: { id: string; reason: string; created_at: string; expires_at: string | null; revoked_at: string | null; created_by: string }[];
  audit: { id: number; action: string; reason: string | null; created_at: string; admin: string }[];
  ip_country_note: string;
}

export default function UserDetail() {
  const { id } = useParams<{ id: string }>();
  const me = useAdmin();
  const { data, error, loading, reload } = useApi<Detail>(`/api/admin/users/${id}`);
  const [dialog, setDialog] = useState<ConfirmSpec | null>(null);
  if (loading && !data) return <Loading />;
  if (!data) return <ErrorNote message={error} />;
  const u = data.user;
  const activeBan = data.bans.find((b) => !b.revoked_at && (!b.expires_at || new Date(b.expires_at) > new Date()));
  const lastSeen = data.sessions.find((s) => s.application_version);
  const act = (action: AdminAction, spec: Omit<ConfirmSpec, "run">, extra?: object): ConfirmSpec => ({
    ...spec, run: async (reason) => { await api(`/api/admin/users/${id}`, { method: "POST", body: { action, reason: reason || undefined, confirm: true, ...extra } }); await reload(); },
  });
  const self = u.discord_id === me.discord_id;
  const activeSessions = data.sessions.filter((s) => !s.revoked_at && new Date(s.expires_at) > new Date());
  return (
    <>
      <PageTitle title={u.display_name ?? u.username} sub={`@${u.username} · Discord ID ${u.discord_id}`} />
      <div className="grid gap-4 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader title="Identity & authorization" />
          <dl className="grid grid-cols-2 gap-x-6 gap-y-3 p-4 text-sm">
            {[
              ["Access", <Badge key="a" tone={activeBan ? "bad" : u.access_status === "authorized" ? "ok" : "warn"}>{activeBan ? "banned" : u.access_status}</Badge>],
              ["Guild member", u.guild_member ? "yes" : "no"], ["Required role", u.role_verified ? "verified" : "not verified"],
              ["Administrator", data.is_admin ? "yes" : "no"], ["First login", fmt(u.created_at)], ["Last login", fmt(u.last_login_at)],
              ["Last heartbeat", fmt(lastSeen?.last_seen_at)], ["App version", lastSeen?.application_version ?? "—"],
            ].map(([k, v], i) => <div key={i}><dt className="text-xs uppercase tracking-wider text-dim">{k}</dt><dd className="mt-0.5">{v}</dd></div>)}
          </dl>
        </Card>
        <Card>
          <CardHeader title="Actions" />
          <div className="flex flex-col gap-2 p-4">
            {self ? <p className="text-sm text-dim">You cannot act on your own account.</p> : (
              <>
                {u.access_status === "authorized"
                  ? <Button onClick={() => setDialog(act("revoke", { title: "Revoke access", body: "Ends all sessions and blocks sign-in until access is allowed again.", needReason: true, danger: true, confirmLabel: "Revoke access" }))}><ShieldOff size={14} />Revoke access</Button>
                  : <Button onClick={() => setDialog(act("allow", { title: "Allow access", body: "Restores this user's ability to sign in (membership is still verified at login).", confirmLabel: "Allow access" }))}><CheckCircle2 size={14} />Allow access</Button>}
                {activeBan
                  ? <Button onClick={() => setDialog(act("unban", { title: "Remove ban", body: "The user may sign in again if they are otherwise authorized.", confirmLabel: "Unban" }))}><Undo2 size={14} />Unban</Button>
                  : <Button variant="danger" onClick={() => setDialog(act("ban", { title: "Ban user", body: "Blocks sign-in and ends all sessions immediately.", needReason: true, danger: true, confirmLabel: "Ban user" }))}><Ban size={14} />Ban user</Button>}
                <Button disabled={!activeSessions.length} onClick={() => setDialog(act("revoke_sessions", { title: "Revoke all sessions", body: `Ends ${activeSessions.length} active session(s). The user can sign in again.`, danger: true, confirmLabel: "Revoke sessions" }))}><LogOut size={14} />Revoke all sessions</Button>
              </>
            )}
          </div>
        </Card>
      </div>

      <Card className="mt-4">
        <CardHeader title="Sign-in history & IP addresses" right={<span className="text-xs text-dim">{data.ip_country_note}</span>} />
        <Table head={["When", "IP address", "Country (approx.)", "Result", "Client", "Version"]} empty={data.logins.length === 0 ? "No sign-ins recorded." : undefined}>
          {data.logins.map((l) => (
            <tr key={l.id}><Td className="text-dim">{fmt(l.created_at)}</Td><Td className="font-mono text-xs">{l.ip ?? "unknown"}</Td><Td>{l.ip_country ?? "—"}</Td>
              <Td><Badge tone={l.result === "success" ? "ok" : "bad"}>{l.result.replace("failed_", "failed: ")}</Badge></Td><Td className="text-dim">{l.client}</Td><Td className="text-dim">{l.application_version ?? "—"}</Td></tr>
          ))}
        </Table>
      </Card>

      <Card className="mt-4">
        <CardHeader title="Application sessions" />
        <Table head={["Created", "Last activity", "Expires", "Version", "IP", "Status"]} empty={data.sessions.length === 0 ? "No sessions." : undefined}>
          {data.sessions.map((s) => {
            const live = !s.revoked_at && new Date(s.expires_at) > new Date();
            return <tr key={s.id}><Td className="text-dim">{fmt(s.created_at)}</Td><Td className="text-dim">{fmt(s.last_seen_at)}</Td><Td className="text-dim">{fmt(s.expires_at)}</Td><Td>{s.application_version ?? "—"}</Td><Td className="font-mono text-xs">{s.ip ?? "—"}</Td><Td><Badge tone={live ? "ok" : "dim"}>{s.revoked_at ? "revoked" : live ? "active" : "expired"}</Badge></Td></tr>;
          })}
        </Table>
      </Card>

      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <Card><CardHeader title="Moderation history" />
          <Table head={["When", "Reason", "By", "State"]} empty={data.bans.length === 0 ? "No bans." : undefined}>
            {data.bans.map((b) => <tr key={b.id}><Td className="text-dim">{fmt(b.created_at)}</Td><Td>{b.reason}</Td><Td className="text-dim">{b.created_by}</Td><Td><Badge tone={b.revoked_at ? "dim" : "bad"}>{b.revoked_at ? "lifted" : "active"}</Badge></Td></tr>)}
          </Table></Card>
        <Card><CardHeader title="Security history (admin actions)" />
          <Table head={["When", "Action", "By", "Reason"]} empty={data.audit.length === 0 ? "No actions." : undefined}>
            {data.audit.map((a) => <tr key={a.id}><Td className="text-dim">{fmt(a.created_at)}</Td><Td>{a.action}</Td><Td className="text-dim">{a.admin}</Td><Td className="text-dim">{a.reason ?? "—"}</Td></tr>)}
          </Table></Card>
      </div>
      <ErrorNote message={error} />
      {dialog && <ConfirmDialog spec={dialog} onClose={() => setDialog(null)} />}
    </>
  );
}
