//! `sitevet.toml`: the `Options` keys plus a `[cli]` table for CLI-only settings.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::Deserialize;

use crate::args::{FailOn, ReportFormat};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliTable {
    pub format: Option<ReportFormat>,
    pub out: Option<PathBuf>,
    pub fail_under: Option<u8>,
    pub fail_on: Option<FailOn>,
    pub baseline: Option<String>,
    pub save: Option<bool>,
}

const CLI_KEYS: &[&str] = &["format", "out", "fail_under", "fail_on", "baseline", "save"];

/// The file at `--config`, or `./sitevet.toml` when it exists, or defaults.
pub fn load(path: Option<&Path>) -> anyhow::Result<(sitevet::Options, CliTable)> {
    let path = match path {
        Some(p) => p.to_path_buf(),
        None if Path::new("sitevet.toml").exists() => PathBuf::from("sitevet.toml"),
        None => return Ok(Default::default()),
    };
    let text = std::fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    parse(&text).with_context(|| format!("in {}", path.display()))
}

pub fn parse(text: &str) -> anyhow::Result<(sitevet::Options, CliTable)> {
    let options = sitevet::Options::from_toml_str(text)?;
    let table: toml::Table = text.parse()?;
    let cli = match table.get("cli") {
        None => CliTable::default(),
        Some(toml::Value::Table(t)) => {
            if let Some(key) = t.keys().find(|k| !CLI_KEYS.contains(&k.as_str())) {
                bail!("invalid option `cli.{key}`: {}", sitevet::unknown_key_reason(key, CLI_KEYS));
            }
            toml::Value::Table(t.clone()).try_into()?
        }
        Some(_) => bail!("`cli` must be a table: [cli]"),
    };
    Ok((options, cli))
}
