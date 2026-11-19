/**
 * Thin, typed bridge to the Rust backend. Outside Tauri (`npm run dev` in a
 * plain browser, for design work only) calls are served by `mockBackend`.
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { mockBackend } from "./mock";
import type { AppErrorShape } from "@/types";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return mockBackend<T>(command, args);
  return tauriInvoke<T>(command, args);
}

export function asAppError(e: unknown): AppErrorShape {
  if (e && typeof e === "object" && "message" in e) return e as AppErrorShape;
  return { message: typeof e === "string" ? e : "Something went wrong." };
}

export function errorMessage(e: unknown): string {
  return asAppError(e).message;
}

export function errorCode(e: unknown): string | undefined {
  return asAppError(e).code;
}
