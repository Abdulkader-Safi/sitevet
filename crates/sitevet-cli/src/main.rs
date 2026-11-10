//! The `sitevet` binary: flags and `sitevet.toml` into `Options`, the library does the audit,
//! this prints progress to stderr and the report to stdout or `--out`.

mod args;
mod commands;
mod config;
mod output;

use std::process::ExitCode;

use clap::Parser;

use args::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    output::init(&cli);
    let result = match cli.command {
        Command::Audit(a) => commands::audit::run(*a),
        Command::Rules(r) => commands::rules::run(&r),
        Command::Init(i) => commands::init::run(&i),
        Command::History(h) => commands::history::run(&h),
        Command::Show(s) => commands::history::show(&s),
        Command::Diff(d) => commands::history::diff(&d),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            output::error(&e);
            ExitCode::from(2)
        }
    }
}
