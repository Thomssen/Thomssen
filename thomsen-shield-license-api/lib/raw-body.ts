import type { VercelRequest } from "@vercel/node";

/** Stripe's webhook signature is computed over the exact raw request
 * bytes - if anything re-serializes the body first (even a
 * logically-identical JSON.stringify of a parsed copy), verification
 * fails. Pairs with `export const config = { api: { bodyParser: false } }`
 * in the webhook handler, which stops Vercel from consuming/parsing the
 * stream before this runs. */
export function readRawBody(req: VercelRequest): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    req.on("data", (chunk: Buffer) => chunks.push(chunk));
    req.on("end", () => resolve(Buffer.concat(chunks)));
    req.on("error", reject);
  });
}
