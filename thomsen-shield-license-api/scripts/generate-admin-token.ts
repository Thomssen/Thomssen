/**
 * Generates a random admin API token for ADMIN_API_TOKEN.
 *
 *   npm run generate-admin-token
 */
import { randomBytes } from "node:crypto";

console.log("ADMIN_API_TOKEN=" + randomBytes(32).toString("base64url"));
