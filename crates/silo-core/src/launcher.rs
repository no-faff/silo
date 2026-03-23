use crate::browser::BrowserEntry;
use std::process::Command;

pub fn launch(entry: &BrowserEntry, url: &str) -> Result<(), String> {
    let parts = shlex::split(&entry.exec).ok_or("invalid Exec line")?;
    let executable = parts.first().ok_or("empty Exec line")?;

    let mut cmd = Command::new(executable);

    for part in &parts[1..] {
        match part.as_str() {
            "%u" | "%U" => {
                cmd.arg(url);
            }
            "%f" | "%F" | "%i" | "%c" | "%k" => continue,
            other => {
                cmd.arg(other);
            }
        };
    }

    if let Some(ref args) = entry.profile_args
        && let Some(parsed) = shlex::split(args) {
            for arg in parsed {
                cmd.arg(arg);
            }
        }

    if !entry.exec.contains("%u") && !entry.exec.contains("%U") {
        cmd.arg(url);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn()
        .map_err(|e| format!("failed to launch {}: {e}", executable))?;

    Ok(())
}
