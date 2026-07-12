// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! Sends a Reset (`R`) command: full reboot, or a single slot.

use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::ble::protocol::{self, Slot};
use crate::commands::common;

pub async fn run(device: Option<&str>, slot: Option<Slot>, timeout: Duration) -> Result<()> {
    let conn = common::connect(device, timeout).await?;
    let command = protocol::build_reset_command(slot);
    let result = tokio::select! {
        r = conn.write_program(&command) => r,
        _ = tokio::signal::ctrl_c() => Err(anyhow!("interrupted")),
    };
    conn.disconnect().await;
    result?;

    match slot {
        Some(s) => println!("Sent reset for slot {s}."),
        None => println!("Sent reset (full reboot)."),
    }
    Ok(())
}
