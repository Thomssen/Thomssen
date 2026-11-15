"use client";
import { useState } from "react";
import { ConfirmDialog, type ConfirmSpec } from "@/components/confirm";
import { PageTitle } from "@/components/shell";
import { Avatar, Badge, Button, Card, ErrorNote, Loading, Table, Td } from "@/components/ui";
import { api, fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";
import Link from "next/link";

interface S { id: string; kind: string; created_at: string; last_seen_at: string; expires_at: string; application_version: string | null; user_id: string; username: string; avatar_url: string | null; status: "active" | "expired" | "revoked" }

export default function SessionsPage() {
  const [all, setAll] = useState(false);
  const { data, error, loading, reload } = useApi<{ sessions: S[] }>(`/api/admin/sessions?state=${all ? "all" : "active"}`, 20000);
  const [dialog, setDialog] = useState<ConfirmSpec | null>(null);
  return (
    <>
      <PageTitle title="Sessions" sub="Application and administrator sessions. Token values are never shown." />
      <div className="mb-4 flex gap-2"><Button variant={all ? "quiet" : "default"} onClick={() => setAll(false)}>Active</Button><Button variant={all ? "default" : "quiet"} onClick={() => setAll(true)}>All</Button></div>
      <ErrorNote message={error} />
      <Card>
        {loading && !data ? <Loading /> : (
          <Table head={["User", "Type", "Created", "Last activity", "Expires", "Version", "Status", ""]} empty={data?.sessions.length === 0 ? "No sessions." : undefined}>
            {data?.sessions.map((s) => (
              <tr key={s.id}>
                <Td><Link href={`/users/${s.user_id}`} className="flex items-center gap-2 hover:text-accent"><Avatar url={s.avatar_url} name={s.username} />{s.username}</Link></Td>
                <Td className="text-dim">{s.kind}</Td><Td className="text-dim">{fmt(s.created_at)}</Td><Td className="text-dim">{fmt(s.last_seen_at)}</Td><Td className="text-dim">{fmt(s.expires_at)}</Td>
                <Td>{s.application_version ?? "—"}</Td>
                <Td><Badge tone={s.status === "active" ? "ok" : "dim"}>{s.status}</Badge></Td>
                <Td>{s.status === "active" && <Button variant="danger" onClick={() => setDialog({ title: "Terminate session", body: `End ${s.username}'s ${s.kind} session now?`, danger: true, confirmLabel: "Terminate", run: async () => { await api("/api/admin/sessions", { method: "POST", body: { id: s.id, confirm: true } }); await reload(); } })}>Terminate</Button>}</Td>
              </tr>
            ))}
          </Table>
        )}
      </Card>
      {dialog && <ConfirmDialog spec={dialog} onClose={() => setDialog(null)} />}
    </>
  );
}
