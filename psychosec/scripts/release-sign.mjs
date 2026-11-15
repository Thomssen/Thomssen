// Signs a release manifest and (optionally) publishes it to the database.
//   node scripts/release-sign.mjs <installer.exe> <version> <download_url> [--notes "text"] [--publish]
// Private key: file at $RELEASE_SIGNING_KEY_FILE (default release-signing/release-private.pem)
// or PEM contents in $RELEASE_SIGNING_KEY. --publish needs $DATABASE_URL.
import { createHash, createPrivateKey, sign } from "node:crypto";
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const flag = (n) => { const i = args.indexOf(n); return i >= 0 ? args.splice(i, 1)[0] : null; };
const publish = flag("--publish");
let notes = null;
{ const i = args.indexOf("--notes"); if (i >= 0) { notes = args[i + 1]; args.splice(i, 2); } }
const [file, version, url] = args;
if (!file || !version || !url || !/^\d+\.\d+\.\d+$/.test(version) || !url.startsWith("https://")) {
  console.error("usage: release-sign.mjs <installer.exe> <version x.y.z> <https download url> [--notes text] [--publish]");
  process.exit(1);
}
const checksum = createHash("sha256").update(readFileSync(file)).digest("hex");
const pem = process.env.RELEASE_SIGNING_KEY ?? readFileSync(process.env.RELEASE_SIGNING_KEY_FILE ?? "release-signing/release-private.pem", "utf8");
// Must match apps/cli/src/update.rs::signed_message and services/api/src/lib/release.ts
const message = `psychosec-release\n${version}\n${url}\n${checksum}`;
const signature = sign(null, Buffer.from(message), createPrivateKey(pem)).toString("base64");
const manifest = { version, download_url: url, checksum_sha256: checksum, signature, notes };
console.log(JSON.stringify(manifest, null, 2));

if (publish) {
  if (!process.env.DATABASE_URL) { console.error("DATABASE_URL is required for --publish"); process.exit(1); }
  const { default: postgres } = await import("postgres");
  const sql = postgres(process.env.DATABASE_URL, { max: 1, prepare: false, ssl: /localhost|127\.0\.0\.1/.test(process.env.DATABASE_URL) ? false : "require" });
  await sql`insert into application_releases (version, download_url, checksum_sha256, signature, notes)
            values (${version}, ${url}, ${checksum}, ${signature}, ${notes})`;
  await sql.end();
  console.error(`published ${version}`);
}
