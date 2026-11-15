# Database

Versioned SQL migrations for Supabase PostgreSQL. Apply in order with the Supabase CLI
(`supabase db push`) or paste into the SQL editor. `scripts/validate-migrations.sh`
applies them to a throwaway PostgreSQL to prove they run cleanly (used in CI).
