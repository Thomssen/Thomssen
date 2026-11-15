# PSYCHOSEC

**PRIVATE OSINT & INVESTIGATION TERMINAL** — a native Windows terminal application for public-source
research, with Discord-restricted access, a private admin dashboard and signed automatic updates.

| Part | Path | Stack |
|---|---|---|
| Windows CLI (`Psychosec.exe`) | `apps/cli` | Rust, Ratatui, Crossterm, Tokio, Reqwest, SQLite |
| Backend API (`api.psychosec.org`) | `services/api` | Next.js route handlers on Vercel, Supabase PostgreSQL |
| Admin dashboard (`admin.psychosec.org`) | `apps/admin` | Next.js, TypeScript, Tailwind, Recharts, Lucide |
| Public site (`psychosec.org`) | `apps/website` | Next.js, TypeScript, Tailwind |
| Database | `database/migrations` | Versioned SQL, RLS enabled |
| Installer | `installer/psychosec.iss` | Inno Setup 6, per-user, no admin rights |
| Release tooling | `scripts/` | Ed25519 manifest signing, Windows build script |

## CLI commands

```
help  modules  status  account  settings  update  clear  login  logout  exit
username search <u>   domain scan <d>   ip lookup <ip>   email check <e>   web inspect <url>   file metadata <path>
case create|open|list|findings|add|delete|rename|note|target|verify|close
report export --format html|json|csv [--out path]
```

Every investigation command performs a real lookup and returns an accurate error if a source is unreachable.
Nothing is simulated. Investigation data is stored locally (SQLite, field-level ChaCha20-Poly1305 encryption,
key in Windows Credential Manager) and is **never** sent to the backend.

Optional keys (read from the operator's environment): `ABUSEIPDB_API_KEY`, `HIBP_API_KEY`.

## Develop

```bash
cargo test -p psychosec                 # CLI unit tests
npm install && npm run typecheck        # all TS workspaces
npm test -w @psychosec/api              # API integration tests (needs PostgreSQL, see docs/testing.md)
npm run build                           # build API, admin, website
```

## Documentation
- [Deployment (Supabase, Discord, Vercel, DNS)](docs/deployment.md)
- [Building and releasing](docs/releasing.md)
- [Security model and source protection](docs/security.md)
- [Testing and known gaps](docs/testing.md)
- [`.env.example`](.env.example)
