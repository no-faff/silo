# Changelog

## 1.1.0

A large release that brings Silo to feature parity with BrowserPicker
on Windows and beyond what any other Linux browser picker offers.

### New features

**Redirect unwrapping**
- Outlook SafeLinks URLs are unwrapped to show the real destination
- Google redirect URLs (`google.com/url?q=...`) are unwrapped
- Nested redirects are handled (SafeLinks wrapping a Google redirect
  wrapping the real URL), up to 10 levels deep
- The picker shows the real domain and a "Unwrapped from a redirect"
  indicator
- Rules match against the unwrapped URL, not the wrapper
- Tracking redirects are bypassed entirely: the tracking service never
  receives the click

**Path-based rules**
- Rules can now include a path: `github.com/no-faff` matches
  `github.com/no-faff/silo` but not `github.com/other`
- Wildcards supported: `*.corp.com`

**Exception rules**
- A rule with "Always show picker" forces the picker for that domain,
  even if a fallback browser is set

**Stale rule detection**
- Rules pointing at a browser that's been hidden or uninstalled show a
  warning icon and "(not found)"

**Suspend rules**
- Toggle to temporarily bypass all rules and show the picker for every
  link

**Office document detection**
- URLs ending in .xlsx, .docx, .pptx (and legacy formats) are labelled
  in the picker as Spreadsheet, Document or Presentation

**Settings redesign**
- Six tabs: Welcome, Browsers, Rules, Open, About, Uninstall
- Welcome tab with guided introduction covering all features
- Browsers tab: hide/show browsers and profiles, add custom browsers,
  re-detect button
- Rules tab: add/edit/delete domain rules, "When no rule matches"
  fallback setting (replaces the old "Always ask" toggle and separate
  fallback browser), suspend toggle, config export/import
- Open tab: paste a URL and open it in any browser, real-time redirect
  unwrapping display, safety check via Google's Transparency Report
- About tab: version (from Cargo.toml), GitHub, report issue, donate
- Uninstall tab: standalone, shows previous default browser name

**Config export/import**
- Export your settings and rules to a JSON file
- Import from a file, with validation (invalid files are rejected, not
  silently applied)
- Toast notification on successful export

**Safety check**
- Check any URL against Google's Transparency Report for known threats
- Opens in whichever browser you've selected on the Open tab

**Custom browsers**
- Add browsers that weren't auto-detected (AppImages, manual installs,
  command-line tools)
- Name, command and optional arguments

**First-run flow**
- Welcome page explains what Silo does and what each tab is for
- Shows which browser is currently the default and confirms it will be
  restored on uninstall
- "Set as default browser" runs in a background thread with progress
  feedback, doesn't freeze the UI
- Close guard warns if you haven't set Silo as default
- Previous default browser is recorded immediately, not just when the
  button is clicked

**Picker improvements**
- Domain displayed prominently at the top
- Redirect indicator shown below domain
- Office document type shown below domain
- Settings gear button in the header bar
- "Always use for..." toggle now has a subtitle explaining it creates
  a rule
- Toggle hidden when there's no domain
- Auto-closes when you click away from it
- Number keys 1-9 and 0 for quick selection

**App icon**
- Silo icon embedded in the binary and auto-installed to the icon theme
- Launcher .desktop file (without browser registration) installed on
  startup so Silo appears in the app launcher before being set as
  default

### Bug fixes

- Uninstall now cleans mimeapps.list to prevent Silo re-registering as
  default when the .desktop file is recreated
- Uninstall falls back to any installed browser if the previous default
  wasn't recorded
- Silo no longer records itself as its own previous default browser
- Settings window no longer closes when deleting a rule or custom
  browser
- Registration failure re-enables the button and shows the error
- Import validates JSON before saving (invalid files show an error
  dialog instead of silently wiping config)
- About tab links use GTK UriLauncher instead of raw xdg-open
- Display::default() guarded against missing display
- install.sh works in both repo and release tarball layouts
- Rules list refreshes when settings window regains focus
- Config not recreated after uninstall

### Other changes

- Version number in About tab read from Cargo.toml
- URL processing debounced on the Open tab (300ms)
- README rewritten with comparison table against Junction,
  Linkquisition, Linklever and BrowserPicker
- desktop_file::find_by_id() for looking up browser names from .desktop
  files without profile suffixes
- config::try_load_from() for validated config loading

## 1.0.0

Initial release.

- Browser picker popup with keyboard shortcuts
- Automatic profile detection for Chromium and Firefox families
- Domain-based rules with "Always use for..." toggle
- Fallback browser setting
- Settings window with rules management
- Register as default browser
- Uninstall with previous browser restore
- Single-instance via D-Bus activation
- App icon
