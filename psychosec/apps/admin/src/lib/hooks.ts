"use client";
import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "./api";

export function useApi<T>(path: string | null, refreshMs = 0) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const load = useCallback(async () => {
    if (!path) return;
    try { setData(await api<T>(path)); setError(null); }
    catch (e) { setError(e instanceof ApiError ? e.message : "Network error"); }
    finally { setLoading(false); }
  }, [path]);
  useEffect(() => { setLoading(true); load(); if (refreshMs) { const t = setInterval(load, refreshMs); return () => clearInterval(t); } }, [load, refreshMs]);
  return { data, error, loading, reload: load };
}
