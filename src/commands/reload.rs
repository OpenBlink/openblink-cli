use std::time::Duration;

use anyhow::Result;

use crate::ble::{manager, protocol};

pub async fn run(device: Option<&str>) -> Result<()> {
    let adapter = manager::first_adapter().await?;
    let peripheral = manager::find_device(&adapter, Duration::from_secs(10), device).await?;
    let conn = manager::Connection::open(peripheral).await?;
    conn.write_program(&protocol::build_reload_command())
        .await?;
    conn.disconnect().await;

    println!("Sent reLoad (L).");
    Ok(())
}
