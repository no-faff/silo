[![Licence: MIT](https://img.shields.io/badge/licence-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![GTK4](https://img.shields.io/badge/GTK4-libadwaita-blue.svg)](https://gnome.pages.gitlab.gnome.org/libadwaita/)

# Silo

**A browser picker for Linux. Click a link, pick which browser opens it.**

Part of the [No Faff](https://github.com/no-faff) suite.

## Why

There are a handful of browser pickers on Linux. [Junction](https://github.com/sonnyp/Junction)
is the most popular but it has no rules and no profile detection.
[Linkquisition](https://github.com/nicoulaj/linkquisition) has rules
but profiles are manual. [Linklever](https://linklever.app) auto-detects
profiles but it's paid and uses a non-native UI. On Windows,
[BrowserPicker](https://github.com/AJMortimer/BrowserPicker) by Andrew
Longmore was the inspiration for this whole project.

I wanted something free, native and thorough: auto-detect every profile
across every browser family, route links with rules, unwrap tracking
redirects, and look like it belongs on a GNOME or KDE desktop.

## What it does

Silo registers as your default browser. It doesn't replace anything. It
sits in the middle so that when you click a link outside a browser (in
an email, a chat app, a document, wherever), Silo pops up and lets you
choose which browser or profile to open it in.

Toggle "Always use for example.com" and that domain goes straight to
your chosen browser next time with no popup.

## How it compares

| | Silo | Junction | Linkquisition | Linklever | BrowserPicker (Win) |
|-|------|----------|---------------|-----------|---------------------|
| Profile auto-detection | All Chromium + Firefox | No | No | Yes | Chrome/Edge only |
| Rules | Domain, path, wildcards, exceptions | No | Regex | Yes | Text match |
| Redirect unwrapping | SafeLinks + Google, nested | No | Plugin | Partial | Yes |
| Fallback browser | Yes | No | Yes | Yes | Yes |
| Config export/import | Yes | No | No | No | No |
| Safety check | Google Transparency Report | No | No | No | No |
| Custom browsers | Yes | No | No | No | No |
| Hide browsers/profiles | Yes | No | No | No | No |
| Native UI | GTK4/libadwaita | GTK4 | GTK3 | Avalonia | WPF |
| Free | MIT | GPLv3 | MIT | Paid | MIT |

Silo is the only free, open-source Linux picker with automatic profile
detection across all Chromium and Firefox-family browsers.

## Profile detection

Silo finds profiles within each browser, not just the browsers
themselves:

- **Chromium family:** Chrome, Edge, Brave, Vivaldi, Opera, Chromium
- **Firefox family:** Firefox, Zen, Floorp, LibreWolf, Waterfox, Mullvad Browser

"Vivaldi - Work" and "Vivaldi - Personal" appear as separate entries in
the picker, each opening the correct profile.

## Redirect unwrapping

Links from email and chat are often wrapped in tracking redirects.
Outlook wraps links through SafeLinks (those long
`safelinks.protection.outlook.com` URLs) and Google wraps them through
`google.com/url`. The real destination is buried inside as a parameter.

Silo detects these, strips the wrapper and shows you where the link
really goes. The picker displays the real domain, and your rules match
against it. This works even when redirects are nested (SafeLinks
wrapping a Google redirect wrapping the actual URL).

Silo also labels Office documents (Word, Excel, PowerPoint) when it
spots them in a URL, so you know what you're about to open.

## Rules

Rules send specific domains straight to a browser without showing the
picker. Create them from the picker ("Always use for...") or from the
Rules tab in settings.

Patterns can be a plain domain like `github.com` or include a path
like `github.com/no-faff`. A path pattern matches anything underneath
it, so `github.com/no-faff` matches `github.com/no-faff/silo` but
not `github.com/other`. Wildcards work too: `*.corp.com`.

Exception rules let you force the picker for a specific domain, even
if you have a fallback browser set. Useful for domains where you
genuinely want to choose each time.

## Installing

Silo needs GTK4 and libadwaita. On Fedora:

```bash
sudo dnf install gtk4 libadwaita
```

On Ubuntu 24.04+:

```bash
sudo apt install libgtk-4-1 libadwaita-1-0
```

Download the [latest release](../../releases/latest), extract and run
the install script:

```bash
tar xzf silo-1.1.0-linux-x86_64.tar.gz
cd silo-1.1.0
./install.sh
```

This copies the binary to `~/.local/bin/` and installs the `.desktop`
file. Launch Silo from your app launcher or run `silo` in a terminal.
On first launch it asks to register as your default browser. Your
previous default is saved and restored if you uninstall.

## Uninstalling

Open settings (launch `silo` with no arguments), go to the Uninstall
tab and click Uninstall. It restores your previous default browser,
removes the config and deletes the binary.

## How it works

1. A link is clicked somewhere on your system
2. Linux calls Silo (it's the registered default browser)
3. If the link is wrapped in a redirect (SafeLinks, Google), Silo
   unwraps it to get the real URL
4. Silo checks your rules. If a rule matches, it opens the assigned
   browser silently
5. If no rule matches, Silo either shows the picker or sends the link
   to whichever browser you've chosen in the "When no rule matches"
   setting on the Rules tab

Keyboard shortcuts in the picker: 1-9 and 0 select a browser, Escape
closes without opening anything.

## Settings

Launch `silo` with no URL to open settings.

- **Welcome:** Introduction, how each tab works, and default browser
  registration
- **Browsers:** All detected browsers and profiles, with toggles to
  hide any you don't use. Add custom browsers for anything not detected
  automatically
- **Rules:** Domain and path rules, the "When no rule matches" fallback
  setting, a suspend toggle to temporarily bypass all rules, and config
  export/import
- **Open:** Paste a URL and open it directly in any browser. Shows
  redirect unwrapping in real time and includes a safety check via
  Google's Transparency Report
- **About:** Version, GitHub link, report issues and donate
- **Uninstall:** Remove Silo and restore your previous default browser

## Building from source

Requires Rust 1.85+ and GTK4/libadwaita development packages:

```bash
# Fedora
sudo dnf install gtk4-devel libadwaita-devel gcc glib2-devel pkgconf-pkg-config

# Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev gcc libglib2.0-dev pkg-config
```

```bash
git clone https://github.com/no-faff/silo.git
cd silo
cargo build --release
strip target/release/silo
```

The binary is at `target/release/silo` (about 1.1 MB stripped).

Run the tests:

```bash
cargo test --workspace
```

## Licence

[MIT](LICENSE)
