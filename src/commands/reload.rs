// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! Sends a reLoad (`L`) command that recycles the mruby/c VM.

use std::time::Duration;

use anyhow::Result;

use crate::ble::protocol;
use crate::commands::common;

pub async fn run(device: Option<&str>, timeout: Duration) -> Result<()> {
    let conn = common::connect(device, timeout).await?;
    let result = conn.write_program(&protocol::build_reload_command()).await;
    conn.disconnect().await;
    result?;

    println!("Sent reLoad (L).");
    Ok(())
}
