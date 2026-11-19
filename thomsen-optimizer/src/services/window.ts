/** Thin wrapper over the Tauri window API for the custom frameless title
 * bar. No-ops outside Tauri so the UI still renders in a plain browser. */
import { isTauri } from "./ipc";

async function current() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

export async function closeWindow(): Promise<void> {
  if (!isTauri()) return;
  await (await current()).close();
}

export async function minimizeWindow(): Promise<void> {
  if (!isTauri()) return;
  await (await current()).minimize();
}

export async function toggleMaximizeWindow(): Promise<void> {
  if (!isTauri()) return;
  await (await current()).toggleMaximize();
}

export async function isWindowMaximized(): Promise<boolean> {
  if (!isTauri()) return false;
  return (await current()).isMaximized();
}

/** Returns an unsubscribe function, wrapped in a promise since the listener
 * registration itself is async. */
export async function onMaximizeChange(cb: (maximized: boolean) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const win = await current();
  const unlisten = await win.onResized(async () => {
    cb(await win.isMaximized());
  });
  return unlisten;
}
