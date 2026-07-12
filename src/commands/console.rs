// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! Streams device console output over BLE until Ctrl-C.

use std::io::Write;
use std::time::Duration;

use anyhow::{bail, Result};
use futures::StreamExt;

use crate::ble::{manager, protocol};
use crate::commands::common;

pub async fn run(device: Option<&str>, timeout: Duration) -> Result<()> {
    let conn = common::connect(device, timeout).await?;
    let result = stream_console(&conn).await;
    conn.disconnect().await;
    result
}

async fn stream_console(conn: &manager::Connection) -> Result<()> {
    conn.subscribe_console().await?;
    let mut notifications = conn.notifications().await?;

    println!("Connected. Streaming console output (press Ctrl-C to stop)...");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nStopping.");
                return Ok(());
            }
            item = notifications.next() => {
                match item {
                    Some(n) if n.uuid == protocol::CONSOLE_CHAR_UUID => {
                        print!("{}", String::from_utf8_lossy(&n.value));
                        let _ = std::io::stdout().flush();
                    }
                    Some(_) => {}
                    None => bail!("device disconnected"),
                }
            }
        }
    }
}
