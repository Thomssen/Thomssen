import { randomBytes } from "node:crypto";

// Excludes 0/O and 1/I/L - a customer has to type this by hand, so the
// alphabet avoids characters that are easy to mis-key or misread.
const ALPHABET = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";

function randomGroup(length: number): string {
  const bytes = randomBytes(length);
  let out = "";
  for (let i = 0; i < length; i++) {
    out += ALPHABET[bytes[i]! % ALPHABET.length];
  }
  return out;
}

export function generateLicenseKey(): string {
  return `TSHIELD-${randomGroup(4)}-${randomGroup(4)}-${randomGroup(4)}`;
}

const LICENSE_KEY_PATTERN = /^TSHIELD-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/;

export function isWellFormedLicenseKey(key: unknown): key is string {
  return typeof key === "string" && LICENSE_KEY_PATTERN.test(key);
}
