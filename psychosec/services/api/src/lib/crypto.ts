import { createHash, randomBytes, timingSafeEqual } from "node:crypto";

export const b64url = (b: Buffer) => b.toString("base64url");
export const randomToken = (bytes = 32) => b64url(randomBytes(bytes));
export const sha256hex = (s: string) => createHash("sha256").update(s).digest("hex");
export const sha256b64url = (s: string) => b64url(createHash("sha256").update(s).digest());
export function safeEqual(a: string, b: string): boolean {
  const x = Buffer.from(a), y = Buffer.from(b);
  return x.length === y.length && timingSafeEqual(x, y);
}
/** Session tokens are prefixed so they are recognisable in secret scanners. */
export const newSessionToken = () => "psy_" + randomToken(32);
