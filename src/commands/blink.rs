use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
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

    let peripheral = match manager::find_device(&adapter, timeout, device).await {
        Ok(p) => p,
        Err(e) => {
            spinner.finish_and_clear();
            return Err(e);
        }
    };
    spinner.set_message("Connecting...");
    let conn = match manager::Connection::open(peripheral).await {
        Ok(c) => c,
        Err(e) => {
            spinner.finish_and_clear();
            return Err(e);
        }
    };
    spinner.finish_and_clear();

    // 3. Transfer and wait for the status, always disconnecting afterwards —
    //    including on error and on Ctrl-C, so the device is not left holding a
    //    dangling connection that blocks later commands.
    let result = tokio::select! {
        r = blink_connected(&conn, &bytecode, slot) => r,
        _ = tokio::signal::ctrl_c() => Err(anyhow!("interrupted")),
    };
    conn.disconnect().await;
    result
}

/// Transfers the bytecode over an established connection and reports the
/// device status.
async fn blink_connected(conn: &manager::Connection, bytecode: &[u8], slot: u8) -> Result<()> {
    let program_notifies = conn.subscribe_program().await?;
    // Open the notification stream before transferring so a status
    // notification arriving immediately after the reLoad is not missed.
    let mut notifications = if program_notifies {
        Some(conn.notifications().await?)
    } else {
        None
    };

    let t1 = Instant::now();
    transfer::transfer(conn, bytecode, slot).await?;
    let transfer_ms = t1.elapsed().as_millis();

    // Wait briefly for the device's status notification, but only when the
    // Program characteristic actually sends one. Some firmware exposes it
    // as write-only, in which case there is no status to wait for.
    let status = match notifications.as_mut() {
        Some(notifications) => tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(n) = notifications.next().await {
                if n.uuid == protocol::PROGRAM_CHAR_UUID {
                    return Some(String::from_utf8_lossy(&n.value).trim().to_string());
                }
            }
            None
        })
        .await
        .ok()
        .flatten(),
        None => None,
    };

    match status {
        Some(msg) if msg.starts_with("ERROR") => bail!("device reported: {msg}"),
        Some(msg) => println!("Transferred and loaded in {transfer_ms} ms ({msg})"),
        None => println!("Transferred and loaded in {transfer_ms} ms"),
    }
    Ok(())
}
