# Silo

Part of the **No Faff** suite of small utilities (github.com/no-faff).

## What it does

A browser picker for Linux. Registers as your default browser, then either
shows a picker popup or silently routes links to the right browser/profile
based on rules. The killer feature is profile detection across all Chromium
and Firefox-family browsers.

## Tech stack

- Rust (edition 2024, requires Rust >= 1.85)
- GTK4 + libadwaita (native GNOME look)
- Three-crate Cargo workspace: silo-core (logic), silo-gui (UI), silo (binary)

## Building

Requires system packages:

```bash
sudo dnf install gtk4-devel libadwaita-devel gcc glib2-devel pkgconf-pkg-config desktop-file-utils
```

```bash
cargo build --workspace
```

## Running

```bash
cargo run --package silo                          # opens settings
cargo run --package silo -- --settings            # opens settings
cargo run --package silo -- "https://example.com" # opens picker
```

## Testing

```bash
cargo test --workspace
```

## Code quality standards

- No speculative code
- No silent failures
- Atomic file writes
- Validate at boundaries
- British English, no Oxford comma, sentence case
- MIT licence

## Key docs

- **Spec:** `docs/superpowers/specs/2026-03-19-silo-design.md`
- **Plan:** `docs/superpowers/plans/2026-03-19-silo-implementation.md`
