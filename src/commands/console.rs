use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use indicatif::ProgressBar;

use crate::ble::{manager, protocol};

pub async fn run(device: Option<&str>, timeout: Duration) -> Result<()> {
    let adapter = manager::first_adapter().await?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Searching for device...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let peripheral = manager::find_device(&adapter, timeout, device).await?;
    let conn = manager::Connection::open(peripheral).await?;
    conn.subscribe_console().await?;
    let mut notifications = conn.notifications().await?;
    spinner.finish_and_clear();

    println!("Connected. Streaming console output (press Ctrl-C to stop)...");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nStopping.");
                break;
            }
            item = notifications.next() => {
                match item {
                    Some(n) if n.uuid == protocol::CONSOLE_CHAR_UUID => {
                        print!("{}", String::from_utf8_lossy(&n.value));
                        let _ = std::io::stdout().flush();
                    }
                    Some(_) => {}
                    None => break,
                }
            }
        }
    }

    conn.disconnect().await;
    Ok(())
}
