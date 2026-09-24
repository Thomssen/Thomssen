/**
 * Signs license activation tokens with Ed25519. The desktop client only
 * ever verifies with the matching public key (embedded in the Rust
 * binary) - it never has anything that could mint a valid token. See
 * `thomsen-shield/src-tauri/src/license/token.rs` for the client side and
 * `scripts/generate-signing-keypair.ts` for producing a fresh keypair.
 *
 * Keys are stored as the raw 32-byte value, base64-encoded, in
 * `LICENSE_SIGNING_PRIVATE_KEY_B64` / `LICENSE_SIGNING_PUBLIC_KEY_B64` -
 * reconstructed here as JWK `OKP`/`Ed25519` KeyObjects, which is the one
 * Node built-in `crypto` import path that accepts raw key bytes directly
 * rather than a DER/PEM wrapper. This exact round trip (Node signs, Rust's
 * ed25519-dalek verifies, using these raw bytes) is covered by
 * `verifySignsCorrectlyForRust` below and was cross-checked against the
 * actual Rust client during development.
 */
import { createPrivateKey, createPublicKey, sign as cryptoSign, verify as cryptoVerify, type KeyObject } from "node:crypto";
import { requireEnv } from "./env.js";

function b64ToB64url(b64: string): string {
  return b64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

let cachedPrivateKey: KeyObject | null = null;
let cachedPublicKey: KeyObject | null = null;

function privateKey(): KeyObject {
  if (!cachedPrivateKey) {
    const d = b64ToB64url(requireEnv("LICENSE_SIGNING_PRIVATE_KEY_B64"));
    const x = b64ToB64url(requireEnv("LICENSE_SIGNING_PUBLIC_KEY_B64"));
    cachedPrivateKey = createPrivateKey({ key: { kty: "OKP", crv: "Ed25519", x, d }, format: "jwk" });
  }
  return cachedPrivateKey;
}

function publicKey(): KeyObject {
  if (!cachedPublicKey) {
    const x = b64ToB64url(requireEnv("LICENSE_SIGNING_PUBLIC_KEY_B64"));
    cachedPublicKey = createPublicKey({ key: { kty: "OKP", crv: "Ed25519", x }, format: "jwk" });
  }
  return cachedPublicKey;
}

export interface TokenPayload {
  licenseKey: string;
  deviceId: string;
  issuedAt: string;
  expiresAt: string;
}

export interface SignedToken {
  payloadJson: string;
  signatureB64: string;
}

/** How long a signed token stays valid before a device must successfully
 * re-validate - this is the client's offline grace period (see the Rust
 * client's module doc on `license::mod`). 14 days: long enough that a
 * laptop offline for a couple of weeks doesn't get locked out, short
 * enough that a disabled/refunded license stops working within a
 * reasonable time even if the device never calls `validate` again. */
const TOKEN_VALIDITY_DAYS = 14;

export function signToken(licenseKey: string, deviceId: string): SignedToken {
  const now = new Date();
  const expires = new Date(now.getTime() + TOKEN_VALIDITY_DAYS * 24 * 60 * 60 * 1000);
  const payload: TokenPayload = {
    licenseKey,
    deviceId,
    issuedAt: now.toISOString(),
    expiresAt: expires.toISOString(),
  };
  // Sign the *exact* string sent over the wire, never a re-serialized
  // copy - matches the client's `SignedToken::verify`, which checks the
  // signature against these same raw bytes rather than trusting
  // re-encoded JSON to be byte-identical.
  const payloadJson = JSON.stringify(payload);
  const signature = cryptoSign(null, Buffer.from(payloadJson), privateKey());
  return { payloadJson, signatureB64: signature.toString("base64") };
}

/** Used only by the setup script / tests to confirm a signature this
 * module produces verifies with the public key alone, the same check the
 * Rust client performs. */
export function verifyToken(token: SignedToken): TokenPayload | null {
  const ok = cryptoVerify(null, Buffer.from(token.payloadJson), publicKey(), Buffer.from(token.signatureB64, "base64"));
  if (!ok) return null;
  return JSON.parse(token.payloadJson) as TokenPayload;
}
