mod ble;
mod cli;
mod commands;
mod compiler;

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

    match cli.command {
        // `compile` does not need BLE or an async runtime.
        Command::Compile { file, output } => commands::compile::run(&file, output.as_deref()),
        // All other commands talk to the device over BLE.
        other => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(run_ble(other))
        }
    }
}

async fn run_ble(command: Command) -> Result<()> {
    match command {
        Command::Scan { timeout } => commands::scan::run(timeout).await,
        Command::Blink { file, device, slot } => {
            commands::blink::run(&file, device.as_deref(), slot).await
        }
        Command::Console { device } => commands::console::run(device.as_deref()).await,
        Command::Reset { device, slot } => commands::reset::run(device.as_deref(), slot).await,
        Command::Reload { device } => commands::reload::run(device.as_deref()).await,
        Command::Compile { .. } => unreachable!("compile is handled synchronously"),
    }
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
