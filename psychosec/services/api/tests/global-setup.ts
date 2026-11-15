import postgres from "postgres";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";

export default async function setup() {
  const url = process.env.TEST_DATABASE_URL ?? "postgres://postgres@localhost:55432/psytest";
  const admin = postgres(url.replace(/\/[^/]+$/, "/postgres"), { max: 1, onnotice: () => {} });
  await admin.unsafe("drop database if exists psytest with (force)");
  await admin.unsafe("create database psytest");
  await admin.end();
  const db = postgres(url, { max: 1, onnotice: () => {} });
  await db.unsafe("do $$ begin create role anon nologin; exception when duplicate_object then null; end $$; do $$ begin create role authenticated nologin; exception when duplicate_object then null; end $$;");
  const dir = path.resolve(__dirname, "../../../database/migrations");
  for (const f of readdirSync(dir).filter((f) => f.endsWith(".sql")).sort()) await db.unsafe(readFileSync(path.join(dir, f), "utf8"));
  await db.end();
}
