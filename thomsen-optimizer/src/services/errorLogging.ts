/** Frontend errors go to the console only - nothing about the user's system is sent anywhere. */
export function reportFrontendError(error: unknown, context?: string): void {
  const msg = error instanceof Error ? `${error.name}: ${error.message}` : String(error);
  console.error(`[thomsen-optimizer] ${context ?? "error"}: ${msg}`);
}
