use std::time::Duration;

use anyhow::{bail, Result};

use crate::ble::{manager, protocol};

pub async fn run(device: Option<&str>, slot: Option<u8>, timeout: Duration) -> Result<()> {
    if let Some(s) = slot {
        if s != 1 && s != 2 {
            bail!("slot must be 1 or 2 (got {s})");
        }
    }

    let adapter = manager::first_adapter().await?;
    let peripheral = manager::find_device(&adapter, timeout, device).await?;
    let conn = manager::Connection::open(peripheral).await?;
    conn.write_program(&protocol::build_reset_command(slot))
        .await?;
    conn.disconnect().await;

    match slot {
        Some(s) => println!("Sent reset for slot {s}."),
        None => println!("Sent reset (full reboot)."),
    }
    Ok(())
}
