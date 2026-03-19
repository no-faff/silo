# Silo - Linux browser picker

**App ID:** `com.nofaff.Silo`
**Part of:** No Faff suite (github.com/no-faff)
**Licence:** MIT

## What it does

A browser picker for Linux. Registers as your default browser, then either
shows a picker popup or silently routes links to the right browser/profile
based on rules. The killer feature is profile detection across all Chromium
and Firefox-family browsers.

## Tech stack

- Rust
- GTK4 + libadwaita (native GNOME look)
- Blueprint (declarative UI markup)
- Meson build system (needed alongside Cargo to install `.desktop` files,
  icons and compile Blueprint UI definitions, which is standard for GNOME apps)

## Distribution

**Primary:** plain binary on GitHub releases (`.tar.gz`), matching the
Windows "single exe, just run it" approach.

**Secondary:** RPM via Fedora COPR for users who want package management.

**Why not Flatpak:** Silo needs to read browser config files across the host
filesystem and launch arbitrary browser processes. Both of these fight
Flatpak's sandbox model. Broad filesystem permissions and host-escape
mechanisms (`flatpak-spawn --host`) would undermine the sandbox and look
suspicious to users. A Flatpak can be added later if there is demand, but
native distribution is a better fit for this app.

## Architecture

### Three crates in one Cargo workspace

- **`silo-core`** - pure logic, no GUI. Browser detection, URL parsing, rule
  matching, config loading/saving. Testable independently.
- **`silo-gui`** - GTK4/libadwaita UI. Picker window, settings window,
  first-run dialog. Depends on `silo-core`.
- **`silo`** - thin binary crate. Parses args, calls into core and gui.

### Runtime behaviour

Single-instance application using `GtkApplication`'s built-in D-Bus
activation. First link click launches Silo. Subsequent link clicks send the
URL to the running instance via the `open` signal. The app stays alive with
minimal idle footprint.

D-Bus name: `com.nofaff.Silo` (matches app ID). `GtkApplication` registers
on the session bus automatically.

Flow per link:

1. Linux launches `silo "https://example.com"` (Silo is the registered
   default browser).
2. If an instance is already running, the URL is sent to it via D-Bus.
3. Load config, check rules.
4. If a rule matches: launch that browser silently.
5. If a fallback is set (and "always ask" is off): launch fallback silently.
6. No rule matched and either no fallback is set or "always ask" is on:
   show the picker window.

"Always ask" controls what happens when no rule matches: show the picker
(always ask = on) or use the fallback browser (always ask = off). Rules
always fire regardless of the "always ask" setting, because rules represent
explicit user choices. This is how SBP works. The separate "suspend rules"
toggle from SBP is not needed for v1.

## Browser and profile detection

### Browser discovery

Scan standard `.desktop` file locations for anything registered as an HTTP
handler (`x-scheme-handler/http`):

- `/usr/share/applications/`
- `/usr/local/share/applications/`
- `~/.local/share/applications/`
- `/var/lib/flatpak/exports/share/applications/`
- `~/.local/share/flatpak/exports/share/applications/`
- `/var/lib/snapd/desktop/applications/` (Snap, common on Ubuntu)

Respect `$XDG_DATA_HOME` (defaulting to `~/.local/share`) and
`$XDG_DATA_DIRS` for user and system `.desktop` file locations.

Read `Name`, `Exec` and `Icon` fields from each. Any browser we have never
heard of still shows up in the picker.

### Profile detection

For browsers we recognise, scan for profiles in native, Flatpak and Snap
config locations:

| Family | Native config | Flatpak config | Snap config | Format |
|---|---|---|---|---|
| Chromium (Chrome, Edge, Brave, Vivaldi, etc.) | `~/.config/[browser]/Local State` | `~/.var/app/[app-id]/config/[browser]/Local State` | `~/snap/[browser]/common/[browser]/Local State` | JSON, `profile.info_cache` has profile names |
| Firefox family (Firefox, Zen, Floorp, LibreWolf, Waterfox, Mullvad) | `~/.mozilla/firefox/profiles.ini` or `~/.config/[browser]/profiles.ini` | `~/.var/app/[app-id]/.mozilla/firefox/profiles.ini` | `~/snap/firefox/common/.mozilla/firefox/profiles.ini` | INI, lists profile directories and names |

Each profile becomes a separate entry in the picker with the correct launch
args (`--profile-directory=` for Chromium, `-P` for Firefox family).

Unknown browsers show up as a single entry with no profile expansion.

### Icon loading

Read the `Icon` field from the `.desktop` file. GTK's icon theme system
resolves it automatically.

### Zero browsers detected

If no browsers are found, show an error dialog: "No browsers found. Install
a browser and try again." with a dismiss button.

## Picker window

### Layout

- Standard libadwaita window, centred on screen (on Wayland the compositor
  controls placement, typically centred; on X11 Silo can request centre
  positioning)
- Header: "Open github.com in" (domain extracted from URL)
- "Always use for github.com" checkbox in the header area
- Body: scrollable flat list of browsers/profiles, each row showing icon,
  name and keyboard shortcut (1-9, then 0 for the tenth)

Entries beyond the tenth have no keyboard shortcut. Click or arrow-key
navigation only.

### Interaction

- Click a row, press its shortcut number or press Enter on the focused row.
  Browser launches, picker hides. If the browser fails to launch (e.g.
  uninstalled since detection), show an error dialog.
- If the checkbox is ticked when you pick, a domain rule is saved.
- The checkbox state persists between sessions.
- Escape or clicking outside the picker closes it without opening anything.
- Arrow keys navigate the list.
- If only one browser/profile exists: open it silently, no picker shown.

### Error handling

If the URL is malformed or empty, show an error dialog with the raw text
that was received and a dismiss button. Do not offer to open broken URLs.

## Rules and config

### Config file

`~/.config/silo/config.json` (follows XDG Base Directory spec, i.e.
`$XDG_CONFIG_HOME/silo/config.json`). Atomic writes (write to `.tmp`,
rename).

### Config structure

```json
{
  "always_ask": false,
  "fallback_browser": {
    "desktop_file": "vivaldi-stable.desktop",
    "args": "--profile-directory=Default"
  },
  "previous_default_browser": "firefox.desktop",
  "remember_choice": true,
  "rules": [
    {
      "domain": "github.com",
      "browser": {
        "desktop_file": "vivaldi-stable.desktop",
        "args": "--profile-directory=Profile 1"
      }
    },
    {
      "domain": "*.corp.company.com",
      "browser": {
        "desktop_file": "firefox.desktop",
        "args": "-P work"
      }
    }
  ]
}
```

Browsers identified by `.desktop` file ID (stable across distros) plus
optional profile args. Domain patterns support wildcards (`*.google.com`).

### Settings window

- Opened by launching Silo without a URL argument (Super > "Silo" > Enter)
- Also accessible via `silo --settings`
- Libadwaita preferences window style
- Sections: rules list (add/remove/edit), fallback browser, "always ask"
  toggle
- No "suspend rules" (redundant with "always ask")

## Default browser registration

### First launch

If no config exists, show the first-run dialog regardless of whether Silo
was launched with a URL argument or not. If a URL was provided, proceed to
the picker after registration.

The dialog:

"Silo needs to be your default browser to work. Set as default?"

Buttons: "Set as default" and "Not now". If the user clicks "Not now":
close the app if no URL was provided; if a URL was provided, show the picker
anyway (it will work for this one link since the OS already routed the URL
to Silo, but future links will go to the old default browser until Silo is
registered). Either way, write a minimal config file (`setup_declined: true`)
so the dialog does not repeat on every launch. The settings window shows a
reminder that Silo is not the default browser, with a button to register.

Before setting, read the current default with `xdg-settings get
default-web-browser` and store it in config as `previous_default_browser`.
Show the user: "Your current default, Firefox, will be restored if you
uninstall Silo" (but only if this is verified to be true during development;
if the system does not auto-restore, say "you'll need to set a new default
browser in your system settings").

Registration: `xdg-settings set default-web-browser com.nofaff.Silo.desktop`.

### Uninstalling

No deregister button in the app. Users uninstall via their package manager.
The messaging about what happens to the default browser depends on verified
system behaviour (tested during development).

### .desktop file

Installed to `~/.local/share/applications/com.nofaff.Silo.desktop` (for
tar.gz installs) or `/usr/share/applications/` (for RPM installs). Declares:

```
MimeType=x-scheme-handler/http;x-scheme-handler/https;
```

`text/html` is not included. Silo is a link router, not a file viewer.
Opening local `.html` files in a picker would be unexpected.

## Logging

Errors log to stderr. When run as a systemd user service or launched from a
desktop environment, these are captured by the journal (`journalctl --user`).
No separate log file for v1.

## Minimum versions

- GTK4 >= 4.12
- libadwaita >= 1.4

These are shipped by Fedora 39+ and Ubuntu 24.04+.

## Testing

### Unit tests (silo-core)

- URL parsing and domain extraction
- Rule matching (exact domain, wildcards, no match)
- Config serialisation/deserialisation
- Browser `.desktop` file parsing
- Profile detection (Chromium and Firefox family, with sample config files
  as test fixtures)

### Integration tests

- Launch Silo with a URL argument, verify correct browser process is spawned
  (mocked)
- Rule creation via "remember my choice" flow
- Config round-trip (save, reload, verify)

### No GUI tests for v1

Testing GTK windows is low value for an app this simple.

## Project structure

```
silo/
├── Cargo.toml              (workspace)
├── meson.build
├── crates/
│   ├── silo-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── browser.rs      (detection, .desktop parsing, profiles)
│   │       ├── config.rs       (load/save, atomic writes)
│   │       ├── rule.rs         (matching, wildcards)
│   │       └── url.rs          (parsing, domain extraction)
│   └── silo-gui/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── picker.rs       (picker window)
│       │   ├── settings.rs     (settings window)
│       │   └── first_run.rs    (registration dialog)
│       └── data/
│           ├── ui/             (Blueprint files)
│           ├── com.nofaff.Silo.desktop
│           └── icons/
├── src/
│   └── main.rs                 (binary entry point)
├── tests/                      (integration tests)
└── CLAUDE.md
```

## Deferred to v2

- SafeLinks/redirect unwrapping
- Drag-and-drop rule reordering
- Filter/search in picker and settings
- Config export/import
- Path pattern matching in rules (e.g. `github.com/gist/*`)
- Rule exceptions (domain with no browser = always show picker)
- Flatpak distribution
- Metainfo XML for software centres

## Relationship to Simple Browser Picker

Silo is a separate app, not a port. It shares the concept and interaction
model but no code. The Windows version (Simple Browser Picker) remains a
separate WPF app in its own repo. Config formats are similar but not
guaranteed compatible.

A cross-platform Tauri rewrite was previously considered but abandoned in
favour of native apps per platform. GTK4/libadwaita gives a genuinely native
GNOME experience that a web-based UI cannot match. The trade-off is no code
sharing between Windows and Linux, but the apps are small enough that this
is acceptable.
