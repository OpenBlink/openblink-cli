// SPDX-FileCopyrightText: Copyright (c) 2025 ViXion Inc. All Rights Reserved.
// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! BLE connection management built on `btleplug`.
//!
//! Implements the device discovery, connection, MTU negotiation and
//! notification flow described by `openblink-webide`
//! (`public_html/js/ble-protocol.js`).

use std::pin::Pin;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use btleplug::api::{
    Central, CharPropFlags, Characteristic, Manager as _, Peripheral as _, ScanFilter,
    ValueNotification, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::Stream;
use tokio::time::sleep;
use uuid::Uuid;

use super::protocol;

/// Maximum time to wait for a single `connect()` to complete before giving up.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// Maximum time to wait for a notification subscription to be confirmed.
const SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Human-readable summary of a discovered peripheral.
pub struct DiscoveredDevice {
    pub name: Option<String>,
    pub address: String,
    pub rssi: Option<i16>,
}

/// Returns the first available Bluetooth adapter.
pub async fn first_adapter() -> Result<Adapter> {
    let manager = Manager::new()
        .await
        .context("failed to initialize the BLE manager")?;
    let adapters = manager
        .adapters()
        .await
        .context("failed to enumerate BLE adapters")?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no Bluetooth adapter found"))
}

async fn is_openblink(peripheral: &Peripheral) -> bool {
    match peripheral.properties().await {
        Ok(Some(props)) => {
            props.services.contains(&protocol::SERVICE_UUID)
                || props
                    .local_name
                    .as_deref()
                    .map(|n| n.starts_with(protocol::NAME_PREFIX))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

/// Scans for OpenBlink devices for the given duration.
pub async fn scan(adapter: &Adapter, timeout: Duration) -> Result<Vec<Peripheral>> {
    adapter
        .start_scan(ScanFilter {
            services: vec![protocol::SERVICE_UUID],
        })
        .await
        .context("failed to start BLE scan")?;
    sleep(timeout).await;
    let peripherals = adapter
        .peripherals()
        .await
        .context("failed to read scan results")?;
    let _ = adapter.stop_scan().await;

    let mut matches = Vec::new();
    for p in peripherals {
        if is_openblink(&p).await {
            matches.push(p);
        }
    }
    Ok(matches)
}

/// Reads a human-readable description of a peripheral.
pub async fn describe(peripheral: &Peripheral) -> DiscoveredDevice {
    let props = peripheral.properties().await.ok().flatten();
    DiscoveredDevice {
        name: props.as_ref().and_then(|p| p.local_name.clone()),
        address: peripheral.address().to_string(),
        rssi: props.as_ref().and_then(|p| p.rssi),
    }
}

/// Scans and selects a device. When `selector` is `None`, the first OpenBlink
/// device is returned; otherwise the name (substring) or address is matched.
pub async fn find_device(
    adapter: &Adapter,
    timeout: Duration,
    selector: Option<&str>,
) -> Result<Peripheral> {
    let devices = scan(adapter, timeout).await?;
    if devices.is_empty() {
        return Err(anyhow!("no OpenBlink devices found"));
    }

    match selector {
        None => Ok(devices.into_iter().next().expect("non-empty")),
        Some(sel) => {
            for p in &devices {
                let info = describe(p).await;
                let name_match = info
                    .name
                    .as_deref()
                    .map(|n| n.contains(sel))
                    .unwrap_or(false);
                let addr_match = info.address.eq_ignore_ascii_case(sel);
                if name_match || addr_match {
                    return Ok(p.clone());
                }
            }
            Err(anyhow!("no connected OpenBlink device matched '{sel}'"))
        }
    }
}

/// An established connection to an OpenBlink device.
pub struct Connection {
    peripheral: Peripheral,
    program: Characteristic,
    console: Characteristic,
    /// Negotiated MTU (device-reported ATT MTU minus 3, or the default).
    pub mtu: usize,
}

impl Connection {
    /// Connects, discovers characteristics and negotiates the MTU.
    pub async fn open(peripheral: Peripheral) -> Result<Self> {
        // Connect unless the peripheral is already connected to this session.
        // The timeout prevents a stuck connect() — e.g. when the device is held
        // by another central and the connection never completes — from blocking
        // forever.
        if !peripheral.is_connected().await.unwrap_or(false) {
            tokio::time::timeout(CONNECT_TIMEOUT, peripheral.connect())
                .await
                .map_err(|_| anyhow!("timed out after {CONNECT_TIMEOUT:?} while connecting"))?
                .context("failed to connect to the device")?;
        }
        peripheral
            .discover_services()
            .await
            .context("failed to discover GATT services")?;

        let chars = peripheral.characteristics();
        let find = |uuid: Uuid| chars.iter().find(|c| c.uuid == uuid).cloned();

        let program = find(protocol::PROGRAM_CHAR_UUID)
            .ok_or_else(|| anyhow!("program characteristic not found on device"))?;
        let console = find(protocol::CONSOLE_CHAR_UUID)
            .ok_or_else(|| anyhow!("console characteristic not found on device"))?;
        let mtu_char = find(protocol::MTU_CHAR_UUID);

        let mtu = negotiate_mtu(&peripheral, mtu_char.as_ref()).await;

        Ok(Self {
            peripheral,
            program,
            console,
            mtu,
        })
    }

    /// Number of payload bytes that fit in a single Data chunk.
    pub fn payload_size(&self) -> usize {
        self.mtu - protocol::DATA_HEADER_SIZE
    }

    /// Writes a command to the Program characteristic.
    pub async fn write_program(&self, data: &[u8]) -> Result<()> {
        // Prefer acknowledged writes (ATT Write Request). A Write Without
        // Response gives btleplug no signal for when the bytes are actually
        // transmitted, so disconnecting right after a transfer can tear the link
        // down in the middle of an L2CAP fragmentation sequence; the device then
        // logs "Ignoring data for unknown channel ID" and drops the program
        // (see zephyrproject-rtos/zephyr#76737). With Write Response each chunk
        // is fully delivered and acknowledged before the next one is sent.
        let write_type = if self.program.properties.contains(CharPropFlags::WRITE) {
            WriteType::WithResponse
        } else {
            WriteType::WithoutResponse
        };
        self.peripheral
            .write(&self.program, data, write_type)
            .await
            .context("BLE write failed")
    }

    /// Subscribes to `characteristic` only when it advertises NOTIFY or
    /// INDICATE, returning whether a subscription was actually established.
    ///
    /// Some OpenBlink firmware exposes the Program characteristic as write-only
    /// (no NOTIFY property, no CCCD). Calling `subscribe` on such a
    /// characteristic never completes, so we skip it instead of hanging.
    async fn subscribe_if_notifiable(
        &self,
        characteristic: &Characteristic,
        name: &str,
    ) -> Result<bool> {
        if !characteristic
            .properties
            .intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)
        {
            return Ok(false);
        }
        tokio::time::timeout(SUBSCRIBE_TIMEOUT, self.peripheral.subscribe(characteristic))
            .await
            .map_err(|_| anyhow!("timed out subscribing to {name} notifications"))?
            .with_context(|| format!("failed to subscribe to {name} notifications"))?;
        Ok(true)
    }

    /// Subscribes to console output notifications. Returns whether the console
    /// characteristic supports notifications.
    pub async fn subscribe_console(&self) -> Result<bool> {
        self.subscribe_if_notifiable(&self.console, "console").await
    }

    /// Subscribes to Program characteristic status notifications (OK/ERROR).
    /// Returns whether the characteristic supports notifications.
    pub async fn subscribe_program(&self) -> Result<bool> {
        self.subscribe_if_notifiable(&self.program, "program").await
    }

    /// Returns the unified notification stream for all subscribed characteristics.
    pub async fn notifications(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ValueNotification> + Send>>> {
        self.peripheral
            .notifications()
            .await
            .context("failed to open the notification stream")
    }

    /// Disconnects from the device (best effort).
    pub async fn disconnect(&self) {
        let _ = self.peripheral.disconnect().await;
    }
}

/// Negotiates the MTU by reading the MTU characteristic (device ATT MTU minus
/// 3). Falls back to the default when the read fails or the value is too small.
async fn negotiate_mtu(peripheral: &Peripheral, mtu_char: Option<&Characteristic>) -> usize {
    if let Some(c) = mtu_char {
        if let Ok(value) = peripheral.read(c).await {
            if value.len() >= 2 {
                let device_mtu = u16::from_le_bytes([value[0], value[1]]) as usize;
                let effective = device_mtu.saturating_sub(3);
                if effective >= protocol::MIN_USABLE_MTU {
                    return effective;
                }
            }
        }
    }
    protocol::DEFAULT_MTU
}
