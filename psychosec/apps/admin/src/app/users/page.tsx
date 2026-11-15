"use client";
import Link from "next/link";
import { useEffect, useState } from "react";
import type { UserRow } from "@psychosec/shared";
import { PageTitle } from "@/components/shell";
import { Avatar, Badge, Button, Card, ErrorNote, Input, Loading, Table, Td } from "@/components/ui";
import { ago, fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";

const statusTone = (s: string, banned: boolean) => (banned ? "bad" : s === "authorized" ? "ok" : "warn") as "bad" | "ok" | "warn";

export default function UsersPage() {
  const [q, setQ] = useState("");
  const [dq, setDq] = useState("");
  const [page, setPage] = useState(0);
  useEffect(() => { const t = setTimeout(() => { setDq(q); setPage(0); }, 300); return () => clearTimeout(t); }, [q]);
  const { data, error, loading } = useApi<{ users: UserRow[] }>(`/api/admin/users?q=${encodeURIComponent(dq)}&page=${page}`);
  return (
    <>
      <PageTitle title="Users" sub="Discord members who have signed in to PSYCHOSEC." />
      <div className="mb-4 max-w-sm"><Input placeholder="Search username, display name or Discord ID" value={q} onChange={(e) => setQ(e.target.value)} /></div>
      <ErrorNote message={error} />
      <Card>
        {loading && !data ? <Loading /> : (
          <Table head={["User", "Discord ID", "Access", "Last login IP", "First login", "Last login", "Last activity", "Sessions"]} empty={data?.users.length === 0 ? "No users match." : undefined}>
            {data?.users.map((u) => (
              <tr key={u.id}>
                <Td><Link href={`/users/${u.id}`} className="flex items-center gap-3 hover:text-accent"><Avatar url={u.avatar_url} name={u.username} /><span>{u.display_name ?? u.username}<span className="block text-xs text-dim">@{u.username}</span></span></Link></Td>
                <Td className="font-mono text-xs text-dim">{u.discord_id}</Td>
                <Td><Badge tone={statusTone(u.access_status, u.banned)}>{u.banned ? "banned" : u.access_status}</Badge></Td>
                <Td className="font-mono text-xs">{u.last_login_ip ?? "—"}</Td>
                <Td className="text-dim">{fmt(u.created_at)}</Td>
                <Td className="text-dim">{fmt(u.last_login_at)}</Td>
                <Td className="text-dim">{ago(u.last_activity_at)}</Td>
                <Td className="tabular-nums">{u.active_sessions}</Td>
              </tr>
            ))}
          </Table>
        )}
      </Card>
      <div className="mt-4 flex items-center justify-end gap-2">
        <Button variant="quiet" disabled={page === 0} onClick={() => setPage(page - 1)}>Previous</Button>
        <span className="text-xs text-dim">Page {page + 1}</span>
        <Button variant="quiet" disabled={(data?.users.length ?? 0) < 50} onClick={() => setPage(page + 1)}>Next</Button>
      </div>
    </>
  );
}
