import postgres from "postgres";
import { env } from "./env";

let _sql: postgres.Sql | undefined;
/** Lazily created pooled connection (use the Supabase transaction pooler URL on Vercel). */
export function sql(): postgres.Sql {
  if (!_sql) {
    _sql = postgres(env.dbUrl, {
      max: 3,
      idle_timeout: 20,
      prepare: false, // required for pgbouncer transaction mode
      ssl: /localhost|127\.0\.0\.1/.test(env.dbUrl) ? false : "require",
      onnotice: () => {},
    });
  }
  return _sql;
}
export async function closeDb() { if (_sql) { await _sql.end(); _sql = undefined; } }
