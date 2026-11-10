//! Terminal output on stderr. Colour follows NO_COLOR, CLICOLOR and whether stderr is a terminal
//! (anstream decides), `--plain` turns it off, and every colour sits next to a status word.

use anstream::eprintln;
use anstyle::{AnsiColor, Style};

use crate::args::Cli;

pub fn init(cli: &Cli) {
    if cli.plain {
        anstream::ColorChoice::Never.write_global();
    }
    let filter = std::env::var("SITEVET_LOG").ok().or_else(|| cli.verbose.then(|| "sitevet=debug".to_string()));
    if let Some(filter) = filter {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
            .with_writer(std::io::stderr)
            .with_ansi(!cli.plain && std::env::var_os("NO_COLOR").is_none())
            .init();
    }
}

pub const GREEN: Style = AnsiColor::Green.on_default();
pub const YELLOW: Style = AnsiColor::Yellow.on_default();
pub const RED: Style = AnsiColor::Red.on_default();
pub const BOLD: Style = Style::new().bold();
pub const DIM: Style = Style::new().dimmed();

/// Status word and colour for a score: 90 and up PASS, 50 and up WARN, below FAIL.
pub fn score_word(score: u8) -> (&'static str, Style) {
    match score {
        90..=100 => ("PASS", GREEN),
        50..=89 => ("WARN", YELLOW),
        _ => ("FAIL", RED),
    }
}

/// `error: <cause>` and a hint for the errors a user can fix.
pub fn error(e: &anyhow::Error) {
    let causes: Vec<String> = e.chain().map(ToString::to_string).collect();
    eprintln!("{RED}{BOLD}error:{BOLD:#}{RED:#} {}", causes.join(": "));
    let hint = match e.chain().find_map(|c| c.downcast_ref::<sitevet::Error>()) {
        Some(sitevet::Error::InvalidUrl { .. }) => Some("use a full URL such as https://example.com"),
        Some(sitevet::Error::StartPageUnreachable { .. }) => {
            Some("check the URL, your connection, and that the site is up")
        }
        Some(sitevet::Error::NotHtml { .. }) => Some("start from an HTML page, such as the home page"),
        Some(sitevet::Error::InvalidOptions { .. }) => {
            Some("check the flag or the key in sitevet.toml (sitevet init writes every key)")
        }
        _ => None,
    };
    if let Some(hint) = hint {
        eprintln!("{DIM}hint: {hint}{DIM:#}");
    }
}
