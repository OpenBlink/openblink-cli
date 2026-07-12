use std::time::Duration;

use anyhow::Result;
use indicatif::ProgressBar;

use crate::ble::manager;

pub async fn run(timeout: Duration) -> Result<()> {
    let adapter = manager::first_adapter().await?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_message(format!(
        "Scanning for OpenBlink devices ({}s)...",
        timeout.as_secs()
    ));
    spinner.enable_steady_tick(Duration::from_millis(100));

    let devices = manager::scan(&adapter, timeout).await?;
    spinner.finish_and_clear();

    if devices.is_empty() {
        println!("No OpenBlink devices found.");
        return Ok(());
    }

    println!("Found {} device(s):", devices.len());
    for p in &devices {
        let info = manager::describe(p).await;
        let name = info.name.unwrap_or_else(|| "(unknown)".to_string());
        let rssi = info
            .rssi
            .map(|r| format!("{r} dBm"))
            .unwrap_or_else(|| "-".to_string());
        println!("  {name:<24} {}  RSSI: {rssi}", info.address);
    }
    Ok(())
}
