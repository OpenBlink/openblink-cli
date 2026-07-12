// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

mod ble;
mod cli;
mod commands;
mod compiler;

use std::future::Future;
use std::time::Duration;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};

use cli::{Cli, Command};

fn main() -> Result<()> {
    // Inject the linked mruby compiler version into `--version` output so it
    // always matches the statically linked library. `clap` requires a
    // `'static` string, so build it once and leak it.
    let version: &'static str = Box::leak(
        format!(
            "{}\nmrbc: {}",
            env!("CARGO_PKG_VERSION"),
            compiler::mrbc_version()
        )
        .into_boxed_str(),
    );
    let matches = Cli::command().version(version).get_matches();
    let cli = Cli::from_arg_matches(&matches)?;

    init_tracing(cli.verbose);

    let timeout = Duration::from_secs(cli.timeout);
    match cli.command {
        // `compile` does not need BLE or an async runtime.
        Command::Compile { file, output } => commands::compile::run(&file, output.as_deref()),
        // All other commands talk to the device over BLE.
        Command::Scan => block_on_ble(commands::scan::run(timeout)),
        Command::Blink { file, device, slot } => block_on_ble(commands::blink::run(
            &file,
            device.as_deref(),
            slot,
            timeout,
        )),
        Command::Console { device } => {
            block_on_ble(commands::console::run(device.as_deref(), timeout))
        }
        Command::Reset { device, slot } => {
            block_on_ble(commands::reset::run(device.as_deref(), slot, timeout))
        }
        Command::Reload { device } => {
            block_on_ble(commands::reload::run(device.as_deref(), timeout))
        }
    }
}

/// Runs a BLE command future on a fresh Tokio runtime.
fn block_on_ble<F: Future<Output = Result<()>>>(fut: F) -> Result<()> {
    tokio::runtime::Runtime::new()?.block_on(fut)
}

fn init_tracing(verbose: u8) {
    use tracing_subscriber::{fmt, EnvFilter};

    let default = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .init();
}
