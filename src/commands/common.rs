// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! Shared helpers for device-connecting commands.

use std::time::Duration;

use anyhow::Result;
use indicatif::ProgressBar;

use crate::ble::manager::{self, Connection};

/// Discovers the target device and opens a connection, showing a spinner
/// while searching. The spinner is cleared on both success and failure.
pub async fn connect(device: Option<&str>, timeout: Duration) -> Result<Connection> {
    let adapter = manager::first_adapter().await?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Searching for device...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let result = async {
        let peripheral = manager::find_device(&adapter, timeout, device).await?;
        spinner.set_message("Connecting...");
        Connection::open(peripheral).await
    }
    .await;
    spinner.finish_and_clear();
    result
}
