# Thomsen Shield

Lightweight network and firewall protection for Windows. Monitor active
connections, detect suspicious traffic, automatically block abusive
connections, and manage Windows Firewall from one clean interface.

Thomsen Shield is a local Windows protection and firewall-management
application. It does not claim to stop every DDoS attack or absorb large
attacks upstream, and it never performs packet flooding, stress testing,
retaliation, or any "counter-attack" against another IP - it is defensive
only.

## Features

- **Dashboard** - protection and firewall status, active connection count,
  blocked IPs, and a small live activity graph. Deliberately minimal.
- **Live Connections** - every active TCP/UDP connection with the owning
  application, remote address, and status. Block an IP, block an
  application, copy an IP, or add one to the allowlist from a connection's
  details.
- **Protection** - automatic detection of connection spikes and repeated
  connections from one IP, with a Low/Normal/High sensitivity control
  (default Normal) tuned to avoid flagging ordinary, bursty applications.
- **Firewall** - view, create, enable/disable, and remove rules, all
  prefixed `ThomsenShield_` so Thomsen Shield only ever touches rules it
  created itself.
- **Blocklist / Allowlist** - every blocked IP with its reason and
  expiration; trusted IPs and ranges (loopback and the RFC1918 private
  ranges are protected automatically) that are never auto-blocked.
- **Events & Logs** - a readable activity history, filterable, exportable
  to TXT or JSON.
- **Lockdown Mode** - an optional, always-reversible stricter mode. A
  "Disable Lockdown" control is always visible while it's active.
- **Licensing** - a one-time €50.00 lifetime purchase, activated with a
  signed token from `thomsen-shield-license-api/` (see that project) and
  cached locally, encrypted with Windows DPAPI.

## Requirements

- Windows 10 or 11, 64-bit
- Administrator rights are requested only for the specific action that
  needs them (creating, removing, or toggling a firewall rule) - never
  just to open or use the app.

## Install using the EXE

Once a release is published, download the installer from this
repository's releases and run it. On first launch it runs a short setup
wizard.

## Build from source

Requires [Node.js](https://nodejs.org) 20+ and the
[Rust toolchain](https://rustup.rs).

```
npm install
npm run release
```

This produces an NSIS installer under
`src-tauri/target/release/bundle/nsis/`. For a dev build with hot reload,
use `npm start` instead (or `npm run dev` to preview the interface in a
plain browser, served by a mock backend - see `src/services/mock.ts`).

## Project layout

```
src/                       React/TypeScript frontend
  components/               shared design-system components (Button, Card,
                             Badge, Toggle, Table, ...) - ported from
                             Thomsen VM's component library
  pages/                     one file per screen
  services/                  typed IPC calls into src-tauri, plus a mock
                              backend for browser-only preview
src-tauri/                 Rust backend
  src/
    network/                 active connection + process enumeration
    firewall/                Windows Firewall (COM) integration, the
                              rules ledger, and the elevated-helper path
                              for rule mutations
    detection/                the spike/repeated-connection engine
    license/                  device id, signed-token verification,
                              DPAPI-encrypted local cache, API client
    elevation.rs               re-launches this exe elevated for one
                                batch of firewall writes, never the whole
                                app
    monitor.rs                  the background polling loop
    commands/                    one file per Tauri command group
website/                   a standalone product-section deliverable for
                            thomssen.dev - see website/README.md
```

## Safety and recovery

Thomsen Shield keeps a ledger of every firewall rule it has ever created.
**Settings → Data → Reset Thomsen Shield Rules** removes every one of
them and restores a clean state. On every launch, the app also
reconciles that ledger against live Windows Firewall state (dropping any
entry for a rule that no longer exists) before creating anything new, so
a crash can never leave behind broken or duplicate rules.

## Author

Built by **Thomsen**
[thomssen.dev](https://thomssen.dev) · [github.com/Thomssen](https://github.com/Thomssen)

## License

Proprietary - €50.00 one-time purchase, lifetime license. Not
open-source.
