// Generates the Ed25519 release-signing key pair (run once, offline).
//   node scripts/release-keygen.mjs
// Private key -> release-signing/release-private.pem (git-ignored; store in a password manager / CI secret).
// Public key (base64, raw 32 bytes) -> set as PSYCHOSEC_UPDATE_PUBKEY when building the CLI.
import { generateKeyPairSync } from "node:crypto";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";

if (existsSync("release-signing/release-private.pem")) {
  console.error("release-signing/release-private.pem already exists; refusing to overwrite.");
  process.exit(1);
}
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
mkdirSync("release-signing", { recursive: true });
writeFileSync("release-signing/release-private.pem", privateKey.export({ type: "pkcs8", format: "pem" }), { mode: 0o600 });
const raw = publicKey.export({ type: "spki", format: "der" }).subarray(-32).toString("base64");
console.log("PSYCHOSEC_UPDATE_PUBKEY=" + raw);
console.log("Private key written to release-signing/release-private.pem. NEVER commit it.");
