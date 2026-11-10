#![allow(clippy::unwrap_used)]
//! The `sitevet` binary against the library's fixture sites.

#[path = "../../sitevet/tests/support/mod.rs"]
mod support;

use assert_cmd::Command;
use support::{Site, ThreadFixture};

fn sitevet() -> Command {
    let mut cmd = Command::cargo_bin("sitevet").unwrap();
    // A fresh folder, so no stray sitevet.toml is picked up.
    // History goes to a throwaway folder, never the real one.
    let home = std::env::temp_dir().join(format!("sitevet-cli-home-{}", std::process::id()));
    cmd.current_dir(std::env::temp_dir())
        .env("SITEVET_HOME", home)
        .env_remove("SITEVET_PSI_KEY")
        .env_remove("SITEVET_LOG");
    cmd
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// FR-CLI-01: JSON on stdout and nothing else.
#[test]
fn json_to_stdout_is_valid_json_only() {
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    let out = sitevet().args(["audit", &fx.url, "--format", "json"]).output().unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["pages"].as_array().unwrap().len(), 5);
    assert!(stderr(&out).contains("5 pages audited"));
}

/// FR-CLI-04: exit 0 passes, 1 fails a threshold, 2 could not run.
#[test]
fn exit_codes() {
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    let run =
        |args: &[&str]| sitevet().arg("audit").arg(&fx.url).args(["--format", "json"]).args(args).output().unwrap();
    assert_eq!(run(&[]).status.code(), Some(0));
    assert_eq!(run(&["--fail-under", "0"]).status.code(), Some(0));
    let under = run(&["--fail-under", "100"]);
    assert_eq!(under.status.code(), Some(1));
    assert!(stderr(&under).contains("threshold failed: score"));
    assert_eq!(run(&["--fail-on", "error"]).status.code(), Some(1), "the basic site has error-severity failures");
    assert_eq!(run(&["--only", "url-structure", "--fail-on", "warning"]).status.code(), Some(0));

    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let down = sitevet().args(["audit", &format!("http://127.0.0.1:{port}/")]).output().unwrap();
    assert_eq!(down.status.code(), Some(2));
    assert!(stderr(&down).contains("error: could not fetch the start page"));
    assert!(stderr(&down).contains("hint:"));
    assert_eq!(sitevet().args(["audit", "not a url"]).output().unwrap().status.code(), Some(2));
    assert_eq!(sitevet().args(["audit", &fx.url, "--max-pages", "10001"]).output().unwrap().status.code(), Some(2));
}

/// FR-CLI-02: a misspelled key names itself and the closest valid key.
#[test]
fn misspelled_config_key_is_named() {
    let dir = std::env::temp_dir().join(format!("sitevet-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sitevet.toml");
    std::fs::write(&file, "max_page = 3\n").unwrap();
    let out = sitevet().args(["audit", "https://example.com", "--config", file.to_str().unwrap()]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("`max_page`: unknown key, did you mean `max_pages`?"), "{}", stderr(&out));

    std::fs::write(&file, "[cli]\nfromat = \"json\"\n").unwrap();
    let out = sitevet().args(["audit", "https://example.com", "--config", file.to_str().unwrap()]).output().unwrap();
    assert!(stderr(&out).contains("`cli.fromat`: unknown key, did you mean `format`?"), "{}", stderr(&out));

    // Values in the file apply, and a flag beats the file.
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    std::fs::write(&file, "max_pages = 2\n[cli]\nformat = \"json\"\n").unwrap();
    let out = sitevet().args(["audit", &fx.url, "--config", file.to_str().unwrap()]).output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["pages"].as_array().unwrap().len(), 2);
    let out =
        sitevet().args(["audit", &fx.url, "--config", file.to_str().unwrap(), "--max-pages", "1"]).output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["pages"].as_array().unwrap().len(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

/// FR-CLI-05: no escape codes with NO_COLOR, even when colour is forced on otherwise.
#[test]
fn no_color_means_no_escape_codes() {
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    let out = sitevet()
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .args(["audit", &fx.url, "--format", "json"])
        .output()
        .unwrap();
    assert!(!stderr(&out).contains('\u{1b}'));
    let forced = sitevet().env("CLICOLOR_FORCE", "1").args(["audit", &fx.url, "--format", "json"]).output().unwrap();
    assert!(stderr(&forced).contains('\u{1b}'), "colour is on when forced, so the NO_COLOR case is meaningful");
    let plain =
        sitevet().env("CLICOLOR_FORCE", "1").args(["--plain", "audit", &fx.url, "--format", "json"]).output().unwrap();
    assert!(!stderr(&plain).contains('\u{1b}'));
    let quiet = sitevet().args(["audit", &fx.url, "--format", "json", "--quiet"]).output().unwrap();
    assert!(stderr(&quiet).is_empty());
}

/// FR-CLI-03: the progress lines and summary on a fixture site.
#[test]
fn stderr_snapshot() {
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    let out =
        sitevet().args(["audit", &fx.url, "--format", "json", "--concurrency", "1", "--no-save"]).output().unwrap();
    let text = stderr(&out).replace(&fx.url, "http://fixture.test").replace(env!("CARGO_PKG_VERSION"), "<version>");
    insta::assert_snapshot!("audit_stderr", text);
}

#[test]
fn report_file_by_extension() {
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    let dir = std::env::temp_dir().join(format!("sitevet-out-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, start) in [("r.html", "<!doctype html>"), ("r.md", "# Audit report"), ("r.json", "{")] {
        let path = dir.join(name);
        let out = sitevet().args(["audit", &fx.url, "--out", path.to_str().unwrap()]).output().unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
        assert!(std::fs::read_to_string(&path).unwrap().starts_with(start), "{name}");
        assert!(stderr(&out).contains("report written to"));
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// FR-CLI-06: `rules` lists every rule.
#[test]
fn rules_command() {
    let out = sitevet().args(["rules", "--format", "json"]).output().unwrap();
    let rules: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rules.len(), sitevet::all_rules().len());
    assert_eq!(rules.iter().filter(|r| r["browser"] == true).count(), 7);
    let table = sitevet().args(["rules", "--category", "legal"]).output().unwrap();
    insta::assert_snapshot!("rules_legal", String::from_utf8_lossy(&table.stdout));
    assert_eq!(sitevet().args(["rules", "--category", "nope"]).output().unwrap().status.code(), Some(2));
}

/// FR-CLI-07: `init` writes the defaults and will not overwrite without --force.
#[test]
fn init_command() {
    let dir = std::env::temp_dir().join(format!("sitevet-init-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let init = || Command::cargo_bin("sitevet").unwrap().current_dir(&dir).arg("init").output().unwrap();
    assert!(init().status.success());
    let written = std::fs::read_to_string(dir.join("sitevet.toml")).unwrap();
    let json = |o: &sitevet::Options| serde_json::to_value(o).unwrap();
    assert_eq!(json(&sitevet::Options::from_toml_str(&written).unwrap()), json(&sitevet::Options::default()));
    assert_eq!(init().status.code(), Some(2), "refuses to overwrite");
    let forced = Command::cargo_bin("sitevet").unwrap().current_dir(&dir).args(["init", "--force"]).output().unwrap();
    assert!(forced.status.success());
    std::fs::remove_dir_all(dir).unwrap();
}
