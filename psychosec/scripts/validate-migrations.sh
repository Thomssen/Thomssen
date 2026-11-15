#!/usr/bin/env bash
# Applies all migrations to a scratch PostgreSQL (needs psql + a server, e.g. the CI service container).
# Usage: PGURL=postgres://postgres:postgres@localhost:5432/postgres scripts/validate-migrations.sh
set -euo pipefail
: "${PGURL:?set PGURL}"
psql "$PGURL" -v ON_ERROR_STOP=1 -c "drop schema public cascade; create schema public; do \$\$ begin create role anon nologin; exception when duplicate_object then null; end \$\$; do \$\$ begin create role authenticated nologin; exception when duplicate_object then null; end \$\$;" >/dev/null
for f in "$(dirname "$0")"/../database/migrations/*.sql; do
  echo "applying $f"; psql "$PGURL" -v ON_ERROR_STOP=1 -q -f "$f"
done
psql "$PGURL" -v ON_ERROR_STOP=1 -q <<'SQL'
select rate_limit_hit('t', 60, 2), rate_limit_hit('t', 60, 2), rate_limit_hit('t', 60, 2);
select purge_old_records();
SQL
echo "migrations OK"
