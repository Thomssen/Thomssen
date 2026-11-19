import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { api } from "@/services/api";
import { isTauri } from "@/services/ipc";
import { MONITORING_ROUTES } from "@/config/navigation";
import { useNavigation } from "./NavigationProvider";
import type { BoosterEvent, BoosterStatus, EngineEvent, Sample } from "@/types";

const HISTORY_LEN = 90;

export interface SamplePoint {
  t: number;
  cpu: number;
  gpu: number | null;
  mem: number;
  diskMBps: number;
  netDownMBps: number;
  netUpMBps: number;
}

interface MonitorValue {
  sample: Sample | null;
  history: SamplePoint[];
  booster: BoosterStatus | null;
  /** Newest booster event, for toasts. */
  lastEvent: { id: number; event: BoosterEvent } | null;
  refreshBooster: () => void;
}

const Ctx = createContext<MonitorValue | null>(null);

function point(s: Sample): SamplePoint {
  return {
    t: s.timestampMs,
    cpu: s.cpuUsage,
    gpu: s.gpu?.usage ?? null,
    mem: s.memoryTotalBytes ? (s.memoryUsedBytes / s.memoryTotalBytes) * 100 : 0,
    diskMBps: (s.diskReadBps + s.diskWriteBps) / 1e6,
    netDownMBps: s.netDownBps / 1e6,
    netUpMBps: s.netUpBps / 1e6,
  };
}

/** Subscribes once to the backend's event stream. The backend decides the
 * cadence (fast only while a monitoring page is open); this provider tells it
 * what is on screen. */
export function MonitorProvider({ children }: { children: ReactNode }) {
  const { route } = useNavigation();
  const [sample, setSample] = useState<Sample | null>(null);
  const [history, setHistory] = useState<SamplePoint[]>([]);
  const [booster, setBooster] = useState<BoosterStatus | null>(null);
  const [lastEvent, setLastEvent] = useState<MonitorValue["lastEvent"]>(null);
  const eventId = useRef(0);

  const pushSample = (s: Sample) => {
    setSample(s);
    setHistory((h) => [...h.slice(-(HISTORY_LEN - 1)), point(s)]);
  };

  const refreshBooster = () => void api.boosterStatus().then(setBooster).catch(() => {});

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let timer: number | undefined;
    let cancelled = false;
    void api.latestSample().then((s) => s && !cancelled && pushSample(s)).catch(() => {});
    refreshBooster();

    if (isTauri()) {
      void import("@tauri-apps/api/event").then(async ({ listen }) => {
        const un = await listen<EngineEvent>("engine://event", (e) => {
          const ev = e.payload;
          if (ev.type === "sample") pushSample(ev.data);
          else if (ev.type === "boosterStatus") setBooster(ev.data);
          else if (ev.type === "booster") setLastEvent({ id: ++eventId.current, event: ev.data });
        });
        if (cancelled) un();
        else unlisten = un;
      });
    } else {
      // Browser preview only: poll the mock.
      timer = window.setInterval(() => void api.latestSample().then((s) => s && pushSample(s)), 1000);
    }
    return () => {
      cancelled = true;
      unlisten?.();
      if (timer) window.clearInterval(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Tell the backend what is on screen so it can lower its own activity.
  useEffect(() => {
    const send = () => void api.setUiState(document.visibilityState === "visible", MONITORING_ROUTES.includes(route)).catch(() => {});
    send();
    document.addEventListener("visibilitychange", send);
    return () => document.removeEventListener("visibilitychange", send);
  }, [route]);

  return <Ctx.Provider value={{ sample, history, booster, lastEvent, refreshBooster }}>{children}</Ctx.Provider>;
}

export function useMonitor(): MonitorValue {
  const c = useContext(Ctx);
  if (!c) throw new Error("useMonitor must be used within <MonitorProvider>");
  return c;
}
