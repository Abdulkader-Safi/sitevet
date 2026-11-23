#![allow(clippy::unwrap_used)]
//! FR-CLI-08 and FR-CLI-09 against a local stand-in for the GitHub releases API. Never GitHub.

#[path = "../../sitevet/tests/support/mod.rs"]
mod support;

use assert_cmd::Command;
use support::{Route, Site, ThreadFixture};

/// A release API that answers with `tag`, plus the basic site to audit.
fn api(tag: &str) -> ThreadFixture {
    let body = format!(r#"{{"tag_name":"{tag}"}}"#);
    ThreadFixture::start(Site::from_dir("basic").route("/latest", Route::html(body).content_type("application/json")))
}

/// Throwaway history, config and Cargo folders, so no real receipt or cache is read.
fn sitevet(fx: &ThreadFixture) -> Command {
    let tmp = std::env::temp_dir().join(format!("sitevet-update-{}", std::process::id()));
    let mut cmd = Command::cargo_bin("sitevet").unwrap();
    cmd.current_dir(std::env::temp_dir())
        .env("SITEVET_HOME", tmp.join("home"))
        .env("XDG_CONFIG_HOME", tmp.join("config"))
        .env("CARGO_HOME", tmp.join("cargo"))
        .env("SITEVET_UPDATE_URL", format!("{}/latest", fx.url.trim_end_matches('/')))
        .env_remove("SITEVET_NO_UPDATE_CHECK");
    cmd
}

fn hits(fx: &ThreadFixture) -> Vec<String> {
    fx.log.lock().unwrap().iter().filter(|h| h.path == "/latest").map(|h| h.user_agent.clone()).collect()
}

#[test]
fn update_says_when_current() {
    let fx = api(&format!("v{}", env!("CARGO_PKG_VERSION")));
    let out = sitevet(&fx).arg("update").output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("is the latest version"));
    assert_eq!(hits(&fx), [format!("sitevet/{}", env!("CARGO_PKG_VERSION"))], "one request, version only");
}

#[test]
fn update_leaves_a_copy_it_cannot_replace() {
    // The test binary lives in target/, which is neither a receipt's folder nor Cargo's bin.
    let fx = api("v99.0.0");
    let out = sitevet(&fx).arg("update").output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("cannot update the copy") && err.contains("cargo install sitevet-cli"), "{err}");
}

#[test]
fn audit_without_a_terminal_never_checks() {
    let fx = api("v99.0.0");
    let out = sitevet(&fx).args(["audit", &fx.url, "--format", "json", "--no-save"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(hits(&fx).is_empty(), "no update request when stdin is not a terminal");
    assert!(!String::from_utf8_lossy(&out.stderr).contains("Update now"));
}
