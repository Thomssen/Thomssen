import { useEffect, useState } from "react";
import { Page } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { Menu } from "@/components/ui/Menu";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { DownloadIcon } from "@/components/icons";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import { exportLogs, listLogs } from "@/services/logs";
import { formatClock, formatDateTime } from "@/lib/format";
import { LOG_CATEGORY_LABELS, type LogCategory, type LogEntry } from "@/types";
import "./EventsLogsPage.css";

type Filter = LogCategory | "all";

const FILTERS: { value: Filter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "protection", label: "Protection" },
  { value: "firewall", label: "Firewall" },
  { value: "blocked", label: "Blocked" },
  { value: "warning", label: "Warnings" },
];

const CATEGORY_TONE: Record<LogCategory, "neutral" | "good" | "bad" | "warn"> = {
  protection: "neutral",
  firewall: "neutral",
  blocked: "bad",
  warning: "warn",
};

export function EventsLogsPage() {
  const toast = useToast();
  const [filter, setFilter] = useState<Filter>("all");
  const [logs, setLogs] = useState<LogEntry[] | null>(null);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    setLogs(null);
    void listLogs(filter).then(setLogs).catch((e) => toast.error(errorMessage(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filter]);

  const doExport = async (format: "txt" | "json") => {
    setExporting(true);
    try {
      const path = await exportLogs(format, filter);
      toast.success(`Exported to ${path}`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setExporting(false);
    }
  };

  const grouped = groupByDay(logs ?? []);

  return (
    <Page
      title="Events & Logs"
      eyebrow="Activity history"
      description="A readable record of what Thomsen Shield has done."
      actions={
        <Menu
          align="end"
          trigger={
            <Button variant="secondary" icon={<DownloadIcon size={15} />} loading={exporting}>
              Export
            </Button>
          }
          items={[
            { label: "Export as TXT", onSelect: () => void doExport("txt") },
            { label: "Export as JSON", onSelect: () => void doExport("json") },
          ]}
        />
      }
    >
      <div className="stack">
        <SegmentedControl options={FILTERS} value={filter} onChange={setFilter} aria-label="Filter events" />

        <Card flush>
          {logs === null ? (
            <LoadingRow label="Loading events" />
          ) : logs.length === 0 ? (
            <EmptyState inset title="No events" description="Nothing has happened in this category yet." />
          ) : (
            <div className="log-list">
              {grouped.map(([day, items]) => (
                <div key={day} className="log-list__group">
                  <p className="log-list__day">{day}</p>
                  {items.map((l) => (
                    <div key={l.id} className="log-row">
                      <span className="log-row__time mono">{formatClock(l.timestamp)}</span>
                      <Badge tone={CATEGORY_TONE[l.category]} dot={false} className="log-row__cat">
                        {LOG_CATEGORY_LABELS[l.category]}
                      </Badge>
                      <span className="log-row__message">{l.message}</span>
                    </div>
                  ))}
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </Page>
  );
}

function groupByDay(logs: LogEntry[]): [string, LogEntry[]][] {
  const map = new Map<string, LogEntry[]>();
  for (const l of logs) {
    const day = formatDateTime(l.timestamp).split(",")[0] ?? "Earlier";
    if (!map.has(day)) map.set(day, []);
    map.get(day)!.push(l);
  }
  return Array.from(map.entries());
}
