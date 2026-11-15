"use client";
import { PageTitle } from "@/components/shell";
import { Badge, Card, CardHeader, ErrorNote, Loading, Table, Td } from "@/components/ui";
import { fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";

interface S { config: Record<string, string | number | boolean>; admins: { id: string; username: string; discord_id: string; role: string; created_at: string }[]; releases: { version: string; published_at: string; withdrawn_at: string | null }[]; you: { role: string } }

export default function SettingsPage() {
  const { data, error, loading } = useApi<S>("/api/admin/settings");
  return (
    <>
      <PageTitle title="Settings" sub="Read-only. Configuration lives in Vercel environment variables." />
      <ErrorNote message={error} />
      {loading && !data ? <Loading /> : data && (
        <div className="space-y-4">
          <Card><CardHeader title="Configuration" />
            <dl className="grid gap-x-6 gap-y-3 p-4 text-sm sm:grid-cols-2">{Object.entries(data.config).map(([k, v]) => <div key={k}><dt className="text-xs uppercase tracking-wider text-dim">{k.replaceAll("_", " ")}</dt><dd className="mt-0.5 font-mono text-xs">{String(v)}</dd></div>)}</dl></Card>
          <Card><CardHeader title="Administrators" />
            <Table head={["User", "Discord ID", "Role", "Since"]}>{data.admins.map((a) => <tr key={a.id}><Td>{a.username}</Td><Td className="font-mono text-xs text-dim">{a.discord_id}</Td><Td><Badge tone="accent">{a.role.replace("_", " ")}</Badge></Td><Td className="text-dim">{fmt(a.created_at)}</Td></tr>)}</Table></Card>
          <Card><CardHeader title="Published releases" />
            <Table head={["Version", "Published", "State"]} empty={data.releases.length === 0 ? "No releases published." : undefined}>{data.releases.map((r) => <tr key={r.version}><Td>{r.version}</Td><Td className="text-dim">{fmt(r.published_at)}</Td><Td><Badge tone={r.withdrawn_at ? "warn" : "ok"}>{r.withdrawn_at ? "withdrawn" : "live"}</Badge></Td></tr>)}</Table></Card>
          <Card><div className="space-y-2 p-4 text-sm text-dim"><p>Login IP addresses are collected from the backend's trusted request context for security and account administration only. They are visible to administrators and purged automatically after the retention period shown above.</p><p>Privacy policy: <a className="text-accent" href="https://psychosec.org/privacy">psychosec.org/privacy</a></p></div></Card>
        </div>
      )}
    </>
  );
}
