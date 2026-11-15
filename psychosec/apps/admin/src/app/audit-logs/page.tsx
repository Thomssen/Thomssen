"use client";
import Link from "next/link";
import { PageTitle } from "@/components/shell";
import { Badge, Card, ErrorNote, Loading, Table, Td } from "@/components/ui";
import { fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";

export default function AuditPage() {
  const { data, error, loading } = useApi<{ logs: { id: number; action: string; reason: string | null; created_at: string; admin: string; target: string | null; target_id: string | null }[] }>("/api/admin/audit-logs");
  return (
    <>
      <PageTitle title="Audit Logs" sub="Every administrative action. Records are append-only." />
      <ErrorNote message={error} />
      <Card>{loading && !data ? <Loading /> : (
        <Table head={["When", "Administrator", "Action", "Affected user", "Reason"]} empty={data?.logs.length === 0 ? "No actions recorded." : undefined}>
          {data?.logs.map((l) => <tr key={l.id}><Td className="text-dim">{fmt(l.created_at)}</Td><Td>{l.admin}</Td><Td><Badge tone="accent">{l.action.replaceAll("_", " ")}</Badge></Td><Td>{l.target_id ? <Link href={`/users/${l.target_id}`} className="hover:text-accent">{l.target}</Link> : "—"}</Td><Td className="text-dim">{l.reason ?? "—"}</Td></tr>)}
        </Table>)}</Card>
    </>
  );
}
