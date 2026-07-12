// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! The Build & Blink command: compile, transfer and reLoad in one step.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;

use crate::ble::protocol::{self, Slot, Status};
use crate::ble::{manager, transfer};
use crate::commands::common;
use crate::compiler;

pub async fn run(file: &Path, device: Option<&str>, slot: Slot, timeout: Duration) -> Result<()> {
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
    let conn = common::connect(device, timeout).await?;

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
async fn blink_connected(conn: &manager::Connection, bytecode: &[u8], slot: Slot) -> Result<()> {
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
                    return Some(protocol::parse_status(&n.value));
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
        Some(Status::Error(msg)) => bail!("device reported: {msg}"),
        Some(Status::Ok(msg)) => println!("Transferred and loaded in {transfer_ms} ms ({msg})"),
        None => println!("Transferred and loaded in {transfer_ms} ms"),
    }
    Ok(())
}
