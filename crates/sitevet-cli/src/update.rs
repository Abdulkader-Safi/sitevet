//! The update check before an audit (FR-CLI-08) and `sitevet update` (FR-CLI-09).
//!
//! One request to the GitHub releases API, at most once a day, cached next to `history.db`.
//! Installing goes the way sitevet was installed: the release installer, `cargo install`, or a
//! printed command when neither applies.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anstream::eprintln;
use anyhow::{Context, bail};

use crate::output::{BOLD, DIM, GREEN};

const CURRENT: &str = env!("CARGO_PKG_VERSION");
const API: &str = "https://api.github.com/repos/Abdulkader-Safi/sitevet/releases/latest";
const RELEASES: &str = "https://github.com/Abdulkader-Safi/sitevet/releases/download";
const DAY: u64 = 24 * 60 * 60;

/// Before an audit: when a newer release is out and someone is at the terminal, offer it.
/// Returns the exit code of the rerun audit when the user updated, else `None` and the audit
/// goes ahead on this version. Nothing here can stop the audit.
pub fn before_audit(enabled: bool, quiet: bool) -> Option<ExitCode> {
    let off = !enabled || quiet || std::env::var_os("SITEVET_NO_UPDATE_CHECK").is_some();
    if off || !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return None;
    }
    let latest = match latest(true) {
        Ok(Some(v)) => v,
        Ok(None) => return None,
        Err(e) => {
            tracing::debug!("update check failed: {e:#}");
            return None;
        }
    };
    eprint!("{BOLD}sitevet {latest} is out{BOLD:#} (you have {CURRENT}). Update now? [y/N] ");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    if std::io::stdin().lock().read_line(&mut answer).is_err() || !matches!(answer.trim(), "y" | "Y" | "yes") {
        return None;
    }
    match install(&latest) {
        Ok(Some(exe)) => Some(rerun(&exe)),
        Ok(None) => None,
        Err(e) => {
            eprintln!("{DIM}update failed, auditing with {CURRENT}: {e:#}{DIM:#}");
            None
        }
    }
}

/// `sitevet update`
pub fn run() -> anyhow::Result<ExitCode> {
    match latest(false)? {
        None => eprintln!("{GREEN}sitevet {CURRENT} is the latest version{GREEN:#}"),
        Some(v) => {
            eprintln!("updating sitevet {CURRENT} to {v}");
            if install(&v)?.is_some() {
                eprintln!("{GREEN}sitevet {v} is installed{GREEN:#}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The latest release when it is newer than this one. `cached` allows an answer under a day old.
fn latest(cached: bool) -> anyhow::Result<Option<String>> {
    let cache = sitevet::History::default_path().and_then(|p| Some(p.parent()?.join("update-check.json")));
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let hit = if cached { cache.as_deref().and_then(|p| fresh(p, now)) } else { None };
    let version = match hit {
        Some(v) => v,
        None => {
            let v = fetch()?;
            if let Some(path) = &cache {
                // A cache that cannot be written only means another request tomorrow.
                let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
                let _ = std::fs::write(path, serde_json::json!({ "checked_at": now, "latest": v }).to_string());
            }
            v
        }
    };
    Ok(newer(&version, CURRENT).then_some(version))
}

/// The cached latest version, when the cache was written less than a day before `now`.
fn fresh(path: &Path, now: u64) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    (now.saturating_sub(v["checked_at"].as_u64()?) < DAY).then(|| v["latest"].as_str().map(str::to_string))?
}

/// The latest release tag without its `v`.
fn fetch() -> anyhow::Result<String> {
    let url = std::env::var("SITEVET_UPDATE_URL").unwrap_or_else(|_| API.to_string());
    let text = runtime()?.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent(format!("sitevet/{CURRENT}"))
            .timeout(Duration::from_secs(3))
            .build()?;
        client.get(&url).header("accept", "application/vnd.github+json").send().await?.error_for_status()?.text().await
    })?;
    let body: serde_json::Value =
        serde_json::from_str(&text).context("the release API sent something other than JSON")?;
    let tag = body["tag_name"].as_str().context("the release has no tag")?;
    Ok(tag.trim_start_matches('v').to_string())
}

fn runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread().enable_all().build()?)
}

/// Is `a` a later x.y.z than `b`? Anything that does not parse is never newer.
fn newer(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Option<(u64, u64, u64)> {
        let mut it = s.split('.').map(|p| p.parse::<u64>().ok());
        Some((it.next()??, it.next()??, it.next()??))
    };
    matches!((parse(a), parse(b)), (Some(a), Some(b)) if a > b)
}

/// Installs `version` and returns the path of the new binary, or `None` when sitevet cannot
/// update this copy and printed what to run instead.
fn install(version: &str) -> anyhow::Result<Option<PathBuf>> {
    let exe = std::env::current_exe().context("could not find the running binary")?;
    let dir = exe.parent().context("the binary has no folder")?;
    if let Some(prefix) = receipt_prefix().filter(|p| dir == p.join("bin") || dir == p.as_path()) {
        with_binary_moved(&exe, || installer(version, &prefix))?;
    } else if cargo_bin().is_some_and(|b| b == dir) {
        with_binary_moved(&exe, || {
            let ok = Command::new("cargo")
                .args(["install", "sitevet-cli", "--locked", "--version", version])
                .status()
                .context("could not run cargo")?
                .success();
            if !ok {
                bail!("cargo install failed");
            }
            Ok(())
        })?;
    } else {
        eprintln!("sitevet cannot update the copy at {}. Install {version} with:", exe.display());
        eprintln!("  cargo install sitevet-cli --locked");
        return Ok(None);
    }
    Ok(Some(exe))
}

/// The install folder from the cargo-dist receipt, when there is one.
fn receipt_prefix() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(x) => PathBuf::from(x),
        None if cfg!(windows) => PathBuf::from(std::env::var_os("LOCALAPPDATA")?),
        None => directories::BaseDirs::new()?.home_dir().join(".config"),
    };
    let text = std::fs::read_to_string(base.join("sitevet-cli").join("sitevet-cli-receipt.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(PathBuf::from(v["install_prefix"].as_str()?))
}

/// `$CARGO_HOME/bin`, else `~/.cargo/bin`.
fn cargo_bin() -> Option<PathBuf> {
    let home = match std::env::var_os("CARGO_HOME") {
        Some(h) => PathBuf::from(h),
        None => directories::BaseDirs::new()?.home_dir().join(".cargo"),
    };
    Some(home.join("bin"))
}

/// Downloads that release's installer and runs it into `prefix`, leaving the PATH alone.
fn installer(version: &str, prefix: &Path) -> anyhow::Result<()> {
    let (name, program, args): (&str, &str, &[&str]) = if cfg!(windows) {
        ("sitevet-cli-installer.ps1", "powershell", &["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
    } else {
        ("sitevet-cli-installer.sh", "sh", &[])
    };
    let url = format!("{RELEASES}/v{version}/{name}");
    let script = runtime()?.block_on(async {
        reqwest::Client::builder()
            .user_agent(format!("sitevet/{CURRENT}"))
            .timeout(Duration::from_secs(60))
            .build()?
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
    })?;
    let mut cmd = Command::new(program);
    cmd.args(args).env("SITEVET_CLI_INSTALL_DIR", prefix).env("SITEVET_CLI_NO_MODIFY_PATH", "1");
    let status = if cfg!(windows) {
        // PowerShell needs a file. %TEMP% is private to the user, and create_new refuses a file
        // someone put there first.
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
        let path = std::env::temp_dir().join(format!("sitevet-{}-{nanos}-{name}", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("could not create {}", path.display()))?;
        file.write_all(script.as_bytes())?;
        drop(file);
        let status = cmd.arg(&path).status();
        let _ = std::fs::remove_file(&path);
        status
    } else {
        // `sh -s` reads the script from stdin, as `curl | sh` does, so nothing touches the disk.
        let mut child =
            cmd.arg("-s").stdin(Stdio::piped()).spawn().with_context(|| format!("could not run {program}"))?;
        child.stdin.take().context("no stdin for the installer")?.write_all(script.as_bytes())?;
        child.wait()
    };
    if !status.with_context(|| format!("could not run {program}"))?.success() {
        bail!("the installer failed");
    }
    Ok(())
}

/// Windows cannot overwrite a running binary, so it is renamed out of the way first and put back
/// if the install fails. Elsewhere the install replaces it in place.
fn with_binary_moved(exe: &Path, install: impl FnOnce() -> anyhow::Result<()>) -> anyhow::Result<()> {
    if !cfg!(windows) {
        return install();
    }
    let old = exe.with_extension("exe.old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old).context("could not move the running binary aside")?;
    let result = install();
    if result.is_err() || !exe.exists() {
        let _ = std::fs::rename(&old, exe);
    }
    result
}

/// Runs the new binary with the same arguments and passes its exit code through.
fn rerun(exe: &Path) -> ExitCode {
    eprintln!();
    let status = Command::new(exe).args(std::env::args_os().skip(1)).env("SITEVET_NO_UPDATE_CHECK", "1").status();
    match status.ok().and_then(|s| s.code()) {
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(2)),
        None => ExitCode::from(2),
    }
}

#[cfg(test)]
mod tests {
    use super::{DAY, fresh, newer};

    #[test]
    fn cache_is_used_for_a_day() {
        let path = std::env::temp_dir().join(format!("sitevet-update-cache-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"checked_at":1000,"latest":"0.7.0"}"#).unwrap();
        assert_eq!(fresh(&path, 1000 + DAY - 1).as_deref(), Some("0.7.0"));
        assert_eq!(fresh(&path, 1000 + DAY), None);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(fresh(&path, 1000), None);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.7.0", "0.6.2"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(!newer("0.6.2", "0.6.2"));
        assert!(!newer("0.6.1", "0.6.2"));
        assert!(!newer("0.7.0-beta.1", "0.6.2"));
        assert!(!newer("garbage", "0.6.2"));
    }
}
