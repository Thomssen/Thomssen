import { invoke, isTauri } from "./ipc";

/** Fire-and-forget report to the Rust side's temporary startup log file -
 * see `installGlobalErrorLogging` below for why this exists. Must never
 * itself throw: a broken error reporter must not become a second crash. */
export function reportFrontendError(message: string): void {
  if (!isTauri()) {
    return;
  }
  void invoke("log_frontend_error", { message }).catch(() => {});
}

/** Catches whatever a page's own <ErrorBoundary> can't: exceptions thrown
 * from event handlers, effects, or timers, and rejected promises nobody
 * awaited. Without this, a release build (no console, no crash reporter)
 * gives zero evidence that any of these happened - the WebView just shows
 * whatever it last painted, which can be nothing at all. */
export function installGlobalErrorLogging(): void {
  window.addEventListener("error", (event) => {
    reportFrontendError(`window.onerror: ${event.message} (${event.filename}:${event.lineno}:${event.colno})\n${event.error?.stack ?? ""}`);
  });

  window.addEventListener("unhandledrejection", (event) => {
    const reason = event.reason;
    const detail = reason instanceof Error ? (reason.stack ?? reason.message) : String(reason);
    reportFrontendError(`unhandledrejection: ${detail}`);
  });
}
