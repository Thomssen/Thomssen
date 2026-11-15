"use client";
import { useState } from "react";
import { PageTitle } from "@/components/shell";
import { Badge, Button, Card, ErrorNote, Loading, Table, Td } from "@/components/ui";
import { fmt } from "@/lib/api";
import { useApi } from "@/lib/hooks";

const TYPES = ["", "login_succeeded", "login_failed", "membership_failed", "session_expired", "logout", "app_launched", "update_checked", "session_revoked"];
const tone = (t: string) => (/failed/.test(t) ? "bad" : t === "login_succeeded" ? "ok" : /revoked|expired|banned/.test(t) ? "warn" : "dim") as "bad" | "ok" | "warn" | "dim";

export default function ActivityPage() {
  const [type, setType] = useState("");
  const { data, error, loading } = useApi<{ events: { id: number; type: string; created_at: string; application_version: string | null; username: string | null; detail: string | null }[] }>(`/api/admin/activity?type=${type}`, 20000);
  return (
    <>
      <PageTitle title="Activity" sub="Operational events only. Investigation content is never collected." />
      <div className="mb-4 flex flex-wrap gap-2">{TYPES.map((t) => <Button key={t} variant={t === type ? "default" : "quiet"} onClick={() => setType(t)}>{t ? t.replaceAll("_", " ") : "all"}</Button>)}</div>
      <ErrorNote message={error} />
      <Card>{loading && !data ? <Loading /> : (
        <Table head={["When", "Event", "User", "Version", "Detail"]} empty={data?.events.length === 0 ? "No events." : undefined}>
          {data?.events.map((e) => <tr key={e.id}><Td className="text-dim">{fmt(e.created_at)}</Td><Td><Badge tone={tone(e.type)}>{e.type.replaceAll("_", " ")}</Badge></Td><Td>{e.username ?? "—"}</Td><Td className="text-dim">{e.application_version ?? "—"}</Td><Td className="text-dim">{e.detail ?? ""}</Td></tr>)}
        </Table>)}</Card>
    </>
  );
}
