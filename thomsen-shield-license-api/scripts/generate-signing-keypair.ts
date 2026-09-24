/**
 * Generates a fresh Ed25519 keypair for signing license activation
 * tokens. Run this once per environment (once for local/dev, once more
 * for production - never reuse the repo's example keys anywhere real).
 *
 *   npm run generate-keypair
 *
 * The private key never gets committed anywhere - paste it straight into
 * Vercel's environment variables (or your local .env) and forget it. The
 * public key half is safe to also paste into the Rust client if you ever
 * rotate keys - see thomsen-shield/src-tauri/src/license/token.rs.
 */
import { generateKeyPairSync } from "node:crypto";

function b64urlToB64(s: string): string {
  return s.replace(/-/g, "+").replace(/_/g, "/");
}

const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubJwk = publicKey.export({ format: "jwk" }) as { x: string };
const privJwk = privateKey.export({ format: "jwk" }) as { d: string };

const publicKeyB64 = Buffer.from(b64urlToB64(pubJwk.x), "base64").toString("base64");
const privateKeyB64 = Buffer.from(b64urlToB64(privJwk.d), "base64").toString("base64");

console.log("Generated a new Ed25519 signing keypair.\n");
console.log("Add these to your Vercel project's environment variables:\n");
console.log(`LICENSE_SIGNING_PRIVATE_KEY_B64=${privateKeyB64}`);
console.log(`LICENSE_SIGNING_PUBLIC_KEY_B64=${publicKeyB64}`);
console.log("\nIf you're rotating keys (not first setup), also update the LICENSE_SIGNING_PUBLIC_KEY");
console.log("bytes in thomsen-shield/src-tauri/src/license/token.rs to this public key,");
console.log("or every existing activation will fail to verify after the rotation.");
