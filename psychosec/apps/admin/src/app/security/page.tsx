"use client";
import Link from "next/link";
import { PageTitle } from "@/components/shell";
import { Badge, Card, CardHeader, ErrorNote, Loading, Table, Td } from "@/components/ui";
import { fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";

interface Sec {
  failed_logins: { id: number; result: string; created_at: string; ip: string | null; ip_country: string | null; client: string; discord_id: string | null; username: string | null }[];
  noisy_ips: { ip: string; failures: number; last_seen: string }[];
  many_ips_7d: { user_id: string; username: string; ips: number }[];
}
export default function SecurityPage() {
  const { data, error, loading } = useApi<Sec>("/api/admin/security", 30000);
  return (
    <>
      <PageTitle title="Security" sub="Failed sign-ins and patterns worth reviewing." />
      <ErrorNote message={error} />
      {loading && !data ? <Loading /> : data && (
        <div className="space-y-4">
          <div className="grid gap-4 lg:grid-cols-2">
            <Card><CardHeader title="IPs with 3+ failures (24h)" />
              <Table head={["IP", "Failures", "Last seen"]} empty={data.noisy_ips.length === 0 ? "None." : undefined}>{data.noisy_ips.map((r) => <tr key={r.ip}><Td className="font-mono text-xs">{r.ip}</Td><Td><Badge tone="bad">{r.failures}</Badge></Td><Td className="text-dim">{fmt(r.last_seen)}</Td></tr>)}</Table></Card>
            <Card><CardHeader title="Accounts using 4+ IPs (7d)" />
              <Table head={["User", "Distinct IPs"]} empty={data.many_ips_7d.length === 0 ? "None." : undefined}>{data.many_ips_7d.map((r) => <tr key={r.user_id}><Td><Link href={`/users/${r.user_id}`} className="hover:text-accent">{r.username}</Link></Td><Td><Badge tone="warn">{r.ips}</Badge></Td></tr>)}</Table></Card>
          </div>
          <Card><CardHeader title="Recent failed sign-ins" />
            <Table head={["When", "User / Discord ID", "IP", "Country (approx.)", "Client", "Reason"]} empty={data.failed_logins.length === 0 ? "No failed sign-ins." : undefined}>
              {data.failed_logins.map((l) => <tr key={l.id}><Td className="text-dim">{fmt(l.created_at)}</Td><Td>{l.username ?? l.discord_id ?? "unknown"}</Td><Td className="font-mono text-xs">{l.ip ?? "unknown"}</Td><Td>{l.ip_country ?? "—"}</Td><Td className="text-dim">{l.client}</Td><Td><Badge tone="bad">{l.result.replace("failed_", "")}</Badge></Td></tr>)}
            </Table></Card>
        </div>
      )}
    </>
  );
}
