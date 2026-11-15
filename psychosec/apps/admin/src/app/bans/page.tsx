"use client";
import Link from "next/link";
import { PageTitle } from "@/components/shell";
import { Avatar, Badge, Card, ErrorNote, Loading, Table, Td } from "@/components/ui";
import { fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";

export default function BansPage() {
  const { data, error, loading } = useApi<{ bans: { id: string; reason: string; created_at: string; expires_at: string | null; revoked_at: string | null; user_id: string; username: string; avatar_url: string | null; created_by: string; active: boolean }[] }>("/api/admin/bans");
  return (
    <>
      <PageTitle title="Bans" sub="Bans are enforced by the backend on every request. Manage them from a user's profile." />
      <ErrorNote message={error} />
      <Card>{loading && !data ? <Loading /> : (
        <Table head={["User", "Reason (internal)", "Banned", "Expires", "By", "State"]} empty={data?.bans.length === 0 ? "No bans recorded." : undefined}>
          {data?.bans.map((b) => <tr key={b.id}><Td><Link href={`/users/${b.user_id}`} className="flex items-center gap-2 hover:text-accent"><Avatar url={b.avatar_url} name={b.username} />{b.username}</Link></Td><Td>{b.reason}</Td><Td className="text-dim">{fmt(b.created_at)}</Td><Td className="text-dim">{b.expires_at ? fmt(b.expires_at) : "never"}</Td><Td className="text-dim">{b.created_by}</Td><Td><Badge tone={b.active ? "bad" : "dim"}>{b.revoked_at ? "lifted" : b.active ? "active" : "expired"}</Badge></Td></tr>)}
        </Table>)}</Card>
    </>
  );
}
