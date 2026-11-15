"use client";
import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import type { Overview } from "@psychosec/shared";
import { PageTitle } from "@/components/shell";
import { Card, CardHeader, ErrorNote, Loading, Stat } from "@/components/ui";
import { useApi } from "@/lib/hooks";

export default function OverviewPage() {
  const { data, error, loading } = useApi<Overview>("/api/admin/overview", 30000);
  return (
    <>
      <PageTitle title="Overview" sub="Live figures from the production database." />
      <ErrorNote message={error} />
      {loading && !data ? <Loading /> : data && (
        <>
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Stat label="Registered users" value={data.total_users} />
            <Stat label="Active (7 days)" value={data.active_users_7d} />
            <Stat label="Valid sessions" value={data.valid_sessions} tone="text-accent" />
            <Stat label="Banned accounts" value={data.banned_accounts} tone={data.banned_accounts ? "text-bad" : undefined} />
            <Stat label="Logins today" value={data.logins_today} tone="text-ok" />
            <Stat label="Failed logins today" value={data.failed_logins_today} tone={data.failed_logins_today ? "text-bad" : undefined} />
            <Stat label="CLI launches today" value={data.cli_launches_today} />
          </div>
          <Card className="mt-6">
            <CardHeader title="Authentication activity (14 days)" />
            <div className="h-72 p-4">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={data.series} margin={{ left: -20 }}>
                  <CartesianGrid stroke="#292B2F" vertical={false} />
                  <XAxis dataKey="day" tickFormatter={(d: string) => d.slice(5)} stroke="#858585" fontSize={11} tickLine={false} />
                  <YAxis allowDecimals={false} stroke="#858585" fontSize={11} tickLine={false} axisLine={false} />
                  <Tooltip cursor={{ fill: "#ffffff08" }} contentStyle={{ background: "#0D0F12", border: "1px solid #292B2F", borderRadius: 6, fontSize: 12 }} />
                  <Bar dataKey="success" name="Successful" stackId="a" fill="#87BDBB" />
                  <Bar dataKey="failed" name="Failed" stackId="a" fill="#D78585" radius={[2, 2, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </Card>
        </>
      )}
    </>
  );
}
