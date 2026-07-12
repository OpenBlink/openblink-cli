use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::ble::{manager, protocol};

pub async fn run(device: Option<&str>, timeout: Duration) -> Result<()> {
    let adapter = manager::first_adapter().await?;
    let peripheral = manager::find_device(&adapter, timeout, device).await?;
    let conn = manager::Connection::open(peripheral).await?;
    let command = protocol::build_reload_command();
    let result = tokio::select! {
        r = conn.write_program(&command) => r,
        _ = tokio::signal::ctrl_c() => Err(anyhow!("interrupted")),
    };
    conn.disconnect().await;
    result?;

    println!("Sent reLoad (L).");
    Ok(())
}
