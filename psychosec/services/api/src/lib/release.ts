/** Must match apps/cli/src/update.rs `signed_message` and scripts/release-sign.mjs. */
export const signedMessage = (r: { version: string; download_url: string; checksum_sha256: string }) =>
  `psychosec-release\n${r.version}\n${r.download_url}\n${r.checksum_sha256.toLowerCase()}`;
