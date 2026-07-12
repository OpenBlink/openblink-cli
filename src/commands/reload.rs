use std::time::Duration;

use anyhow::Result;

use crate::ble::{manager, protocol};

pub async fn run(device: Option<&str>, timeout: Duration) -> Result<()> {
    let adapter = manager::first_adapter().await?;
    let peripheral = manager::find_device(&adapter, timeout, device).await?;
    let conn = manager::Connection::open(peripheral).await?;
    let result = conn.write_program(&protocol::build_reload_command()).await;
    conn.disconnect().await;
    result?;

    println!("Sent reLoad (L).");
    Ok(())
}
