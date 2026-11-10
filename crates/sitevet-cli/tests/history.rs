#![allow(clippy::unwrap_used)]
//! M5: run history through the CLI. Each test gets its own SITEVET_HOME.

#[path = "../../sitevet/tests/support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use assert_cmd::Command;
use support::{Site, ThreadFixture};

fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sitevet-history-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn sitevet(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("sitevet").unwrap();
    cmd.current_dir(std::env::temp_dir()).env("SITEVET_HOME", home).env_remove("SITEVET_PSI_KEY");
    cmd
}

fn out(cmd: &mut Command) -> (i32, String, String) {
    let o = cmd.output().unwrap();
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn runs(home: &Path) -> Vec<i64> {
    sitevet::History::open(Some(home.join("history.db")))
        .unwrap()
        .list(None, 100)
        .unwrap()
        .iter()
        .map(|r| r.id)
        .collect()
}

/// FR-HIST-01: two runs make two rows, a --no-save run none.
#[test]
fn audits_are_saved_unless_no_save() {
    let h = home("save");
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    for _ in 0..2 {
        let (code, _, err) = out(sitevet(&h).args(["audit", &fx.url, "--format", "json"]));
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("saved as run"), "{err}");
    }
    let (_, _, err) = out(sitevet(&h).args(["audit", &fx.url, "--format", "json", "--no-save"]));
    assert!(!err.contains("saved as run"));
    assert_eq!(runs(&h), [2, 1]);
}

/// FR-HIST-02: newest first, filtered by site, and an empty state.
#[test]
fn history_lists_newest_first() {
    let h = home("list");
    let (code, stdout, err) = out(sitevet(&h).arg("history"));
    assert_eq!((code, stdout.as_str()), (0, ""));
    assert!(err.contains("No runs saved yet. Run sitevet audit <url>."));
    let a = ThreadFixture::start(Site::from_dir("basic"));
    let b = ThreadFixture::start(Site::from_dir("arabic"));
    for url in [&a.url, &b.url, &a.url] {
        sitevet(&h).args(["audit", url, "--format", "json"]).output().unwrap();
    }
    let (_, list, _) = out(sitevet(&h).arg("history"));
    let ids: Vec<&str> = list.lines().skip(1).map(|l| l.split_whitespace().next().unwrap()).collect();
    assert_eq!(ids, ["3", "2", "1"]);
    assert!(list.starts_with("ID  DATE"), "{list}");
    let (_, only_a, _) = out(sitevet(&h).args(["history", &format!("{}/about.html", a.url)]));
    assert_eq!(only_a.lines().count(), 3, "header plus runs 3 and 1:\n{only_a}");
}

/// FR-HIST-03: `show` gives back exactly what the audit printed.
#[test]
fn show_renders_the_stored_run() {
    let h = home("show");
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    let (_, json, _) = out(sitevet(&h).args(["audit", &fx.url, "--format", "json"]));
    let (code, shown, _) = out(sitevet(&h).args(["show", "1", "--format", "json"]));
    assert_eq!(code, 0);
    assert_eq!(shown, json);
    let (_, html, _) = out(sitevet(&h).args(["show", "1", "--format", "html"]));
    assert!(html.starts_with("<!doctype html>"));
    let (code, _, err) = out(sitevet(&h).args(["show", "42"]));
    assert_eq!(code, 2);
    assert!(err.contains("no run 42") && err.contains("sitevet history lists"), "{err}");
}

/// FR-HIST-04: fixing one rule between two runs shows it as fixed, with the category change.
#[test]
fn diff_and_baseline_show_the_fix() {
    let h = home("diff");
    let broken = Site::new().page("/", "<p>No title on this page.</p>");
    let fixed = Site::new().page("/", "<title>A title that is long enough to pass</title><p>Fixed.</p>");
    let fx = ThreadFixture::start_with_alt(broken, fixed);
    sitevet(&h).args(["audit", &fx.url, "--format", "json"]).output().unwrap();
    fx.switched.store(true, Ordering::SeqCst);
    sitevet(&h).args(["audit", &fx.url, "--format", "json"]).output().unwrap();

    let (code, json, _) = out(sitevet(&h).args(["diff", "1", "2", "--format", "json"]));
    assert_eq!(code, 0);
    let d: serde_json::Value = serde_json::from_str(&json).unwrap();
    let fixed_rules: Vec<&str> =
        d["fixed"].as_array().unwrap().iter().map(|f| f["rule_id"].as_str().unwrap()).collect();
    assert!(fixed_rules.contains(&"core-seo/title-present"), "{fixed_rules:?}");
    let core = d["categories"].as_array().unwrap().iter().find(|c| c["category"] == "core-seo").unwrap();
    assert!(core["after"].as_u64().unwrap() > core["before"].as_u64().unwrap());
    assert_eq!(d["baseline"]["run_id"], 1);

    let (_, text, _) = out(sitevet(&h).args(["--plain", "diff", "1", "2"]));
    assert!(text.starts_with("run 1 -> run 2") && text.contains("core-seo/title-present"), "{text}");

    // --baseline last on a third run compares against run 2 and puts a changes panel in the report.
    let (_, md, err) = out(sitevet(&h).args(["audit", &fx.url, "--format", "md", "--baseline", "last"]));
    assert!(md.contains("## Changes since run 2"), "{md}");
    assert!(err.contains("changes since run 2: score 0, 0 fixed, 0 new issues"), "{err}");
    let (_, md, _) = out(sitevet(&h).args(["audit", &fx.url, "--format", "md", "--baseline", "1", "--no-save"]));
    assert!(md.contains("## Changes since run 1") && md.contains("core-seo/title-present"), "{md}");
    let (code, _, _) = out(sitevet(&h).args(["audit", &fx.url, "--baseline", "99"]));
    assert_eq!(code, 2, "an unknown baseline run fails before the crawl");
}

/// FR-HIST-05: delete and prune ask first, and prune keeps the newest runs.
#[test]
fn delete_and_prune() {
    let h = home("prune");
    let fx = ThreadFixture::start(Site::from_dir("arabic"));
    for _ in 0..4 {
        sitevet(&h).args(["audit", &fx.url, "--format", "json"]).output().unwrap();
    }
    // No terminal and no --yes: refuse.
    let (code, _, err) = out(sitevet(&h).args(["history", "delete", "1"]));
    assert_eq!(code, 2);
    assert!(err.contains("pass --yes"), "{err}");
    assert_eq!(runs(&h).len(), 4);
    let (code, _, _) = out(sitevet(&h).args(["history", "delete", "1", "--yes"]));
    assert_eq!(code, 0);
    assert_eq!(runs(&h), [4, 3, 2]);
    let (code, _, err) = out(sitevet(&h).args(["history", "prune", "--keep", "2", "--yes"]));
    assert_eq!(code, 0);
    assert!(err.contains("deleted 1 run"), "{err}");
    assert_eq!(runs(&h), [4, 3]);
    assert_eq!(out(sitevet(&h).args(["history", "delete", "1", "--yes"])).0, 2, "unknown run ID");
}
