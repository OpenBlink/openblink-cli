use std::io::Write;
use std::time::Duration;

use anyhow::{bail, Result};
use futures::StreamExt;
use indicatif::ProgressBar;

use crate::ble::{manager, protocol};

pub async fn run(device: Option<&str>, timeout: Duration) -> Result<()> {
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
    let conn = match manager::Connection::open(peripheral).await {
        Ok(c) => c,
        Err(e) => {
            spinner.finish_and_clear();
            return Err(e);
        }
    };
    spinner.finish_and_clear();

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
