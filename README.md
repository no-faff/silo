[![Licence: MIT](https://img.shields.io/badge/licence-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![GTK4](https://img.shields.io/badge/GTK4-libadwaita-blue.svg)](https://gnome.pages.gitlab.gnome.org/libadwaita/)

# Silo

**A browser picker for Linux. Click a link, pick which browser opens it.**

Part of the [No Faff](https://github.com/no-faff) suite.

## Why

On Windows I was using [BrowserPicker](https://github.com/AJMortimer/BrowserPicker)
by Andrew Longmore and liked it. I wanted the same thing on Linux, with
broader profile detection and a native GNOME look.

## What it does

Silo registers as your default browser. Don't worry, it doesn't replace
anything. It just sits in the middle so that when you click a link
outside a browser (in an email, a chat app, a document, wherever), Silo
pops up and lets you choose which browser or profile to open it in.

Toggle "Always use for example.com" and that domain goes straight to
your chosen browser next time with no popup.

## Profile detection

Silo detects profiles within each browser, not just the browsers
themselves:

- **Chromium family:** Chrome, Edge, Brave, Vivaldi, Opera, Chromium
- **Firefox family:** Firefox, Zen, Floorp, LibreWolf, Waterfox, Mullvad Browser

"Vivaldi - Work" and "Vivaldi - Personal" appear as separate entries.

## Redirect unwrapping

Links from email and chat are often wrapped in tracking redirects
(Outlook SafeLinks, Google redirects). Silo strips these and shows you
where the link really goes. It also labels Office documents (Word, Excel,
PowerPoint) when it spots them in a URL.

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
tar xzf silo-1.0.0-linux-x86_64.tar.gz
cd silo-1.0.0
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
3. Silo checks your rules. If a rule matches the domain, it opens that
   browser silently
4. If no rule matches, Silo either shows the picker or sends the link
   to whichever browser you've chosen in the "When no rule matches"
   setting on the Rules tab
5. Keyboard shortcuts: 1-9 and 0 to pick a browser, Escape to close

## Settings

Launch `silo` with no URL to open settings.

- **Welcome:** Introduction and default browser setup
- **Browsers:** Hide browsers or profiles you don't use. Add custom
  browsers that weren't detected automatically (e.g. a browser
  installed outside the standard locations, or a command-line tool
  you want to open links with)
- **Rules:** Domain rules, the "When no rule matches" setting, suspend
  rules toggle, and config export/import
- **Open:** Paste a URL and open it in any browser. Includes redirect
  unwrapping and a safety check via Google's Transparency Report
- **About:** Version, links and donate
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
