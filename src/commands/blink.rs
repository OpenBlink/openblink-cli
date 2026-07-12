use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use futures::StreamExt;
use indicatif::ProgressBar;

use crate::ble::{manager, protocol, transfer};
use crate::compiler;

pub async fn run(file: &Path, device: Option<&str>, slot: u8, timeout: Duration) -> Result<()> {
    // 1. Compile the Ruby source to bytecode.
    let source =
        fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))?;
    let t0 = Instant::now();
    let bytecode = compiler::compile_ruby(&source, &file.to_string_lossy())
        .with_context(|| format!("failed to compile {}", file.display()))?;
    println!(
        "Compiled {} ({} bytes) in {} ms",
        file.display(),
        bytecode.len(),
        t0.elapsed().as_millis()
    );

    // 2. Discover and connect to the device.
    let adapter = manager::first_adapter().await?;
    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Searching for device...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let peripheral = manager::find_device(&adapter, timeout, device).await?;
    spinner.set_message("Connecting...");
    let conn = manager::Connection::open(peripheral).await?;
    let program_notifies = conn.subscribe_program().await?;
    spinner.finish_and_clear();

    // 3. Transfer the bytecode and reLoad.
    let t1 = Instant::now();
    transfer::transfer(&conn, &bytecode, slot).await?;
    let transfer_ms = t1.elapsed().as_millis();

    // 4. Wait briefly for the device's status notification, but only when the
    //    Program characteristic actually sends one. Some firmware exposes it
    //    as write-only, in which case there is no status to wait for.
    let status = if program_notifies {
        let mut notifications = conn.notifications().await?;
        tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(n) = notifications.next().await {
                if n.uuid == protocol::PROGRAM_CHAR_UUID {
                    return Some(String::from_utf8_lossy(&n.value).trim().to_string());
                }
            }
            None
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    conn.disconnect().await;

    match status {
        Some(msg) if msg.starts_with("ERROR") => bail!("device reported: {msg}"),
        Some(msg) => println!("Transferred and loaded in {transfer_ms} ms ({msg})"),
        None => println!("Transferred and loaded in {transfer_ms} ms"),
    }
    Ok(())
}
