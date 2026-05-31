// SPDX-FileCopyrightText: Copyright (c) 2025 ViXion Inc. All Rights Reserved.
// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! OpenBlink BLE protocol constants and command builders.
//!
//! Ported from `openblink-webide` (`public_html/js/config.js` and
//! `public_html/js/ble-protocol.js`).

use uuid::{uuid, Uuid};

/// Primary GATT service exposed by OpenBlink devices.
pub const SERVICE_UUID: Uuid = uuid!("227da52c-e13a-412b-befb-ba2256bb7fbe");
/// Characteristic used to write D/P/L/R commands and receive status notifications.
pub const PROGRAM_CHAR_UUID: Uuid = uuid!("ad9fdd56-1135-4a84-923c-ce5a244385e7");
/// Characteristic that streams device console output via notifications.
pub const CONSOLE_CHAR_UUID: Uuid = uuid!("a015b3de-185a-4252-aa04-7a87d38ce148");
/// Characteristic that reports the negotiated ATT MTU (uint16, little-endian).
pub const MTU_CHAR_UUID: Uuid = uuid!("ca141151-3113-448b-b21a-6a6203d253ff");

/// Advertised device name prefix.
pub const NAME_PREFIX: &str = "OpenBlink";

/// Fallback MTU when negotiation fails.
pub const DEFAULT_MTU: usize = 20;
/// Size of the Data command header (version + command + offset + size).
pub const DATA_HEADER_SIZE: usize = 6;
/// Size of the Program command packet.
pub const PROGRAM_HEADER_SIZE: usize = 8;
/// Minimum negotiated MTU that still carries at least one payload byte.
pub const MIN_USABLE_MTU: usize = 7;

/// Maximum compiled program size, bounded by the 16-bit length/offset fields.
pub const MAX_PROGRAM_SIZE: usize = u16::MAX as usize;

/// Protocol version byte that prefixes every command.
pub const PROTOCOL_VERSION: u8 = 0x01;
pub const CMD_DATA: u8 = b'D';
pub const CMD_PROGRAM: u8 = b'P';
pub const CMD_LOAD: u8 = b'L';
pub const CMD_RESET: u8 = b'R';

/// Builds a Data (`D`) command carrying a chunk of bytecode at `offset`.
pub fn build_data_chunk(offset: u16, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(DATA_HEADER_SIZE + payload.len());
    buf.push(PROTOCOL_VERSION);
    buf.push(CMD_DATA);
    buf.extend_from_slice(&offset.to_le_bytes());
    buf.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    buf.extend_from_slice(payload);
    buf
}

/// Builds a Program (`P`) command with the total length, CRC16 and target slot.
pub fn build_program_command(length: u16, crc16: u16, slot: u8) -> Vec<u8> {
    let mut buf = Vec::with_capacity(PROGRAM_HEADER_SIZE);
    buf.push(PROTOCOL_VERSION);
    buf.push(CMD_PROGRAM);
    buf.extend_from_slice(&length.to_le_bytes());
    buf.extend_from_slice(&crc16.to_le_bytes());
    buf.push(slot);
    buf.push(0x00);
    buf
}

/// Builds a reLoad (`L`) command that reloads the mruby/c VM.
pub fn build_reload_command() -> Vec<u8> {
    vec![PROTOCOL_VERSION, CMD_LOAD]
}

/// Builds a Reset (`R`) command. With `slot`, resets that slot; without it,
/// the device performs a full reboot.
pub fn build_reset_command(slot: Option<u8>) -> Vec<u8> {
    match slot {
        Some(s) => vec![PROTOCOL_VERSION, CMD_RESET, s],
        None => vec![PROTOCOL_VERSION, CMD_RESET],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_chunk_layout() {
        let chunk = build_data_chunk(0x0102, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(chunk[0], 0x01); // version
        assert_eq!(chunk[1], b'D');
        assert_eq!(&chunk[2..4], &[0x02, 0x01]); // offset LE
        assert_eq!(&chunk[4..6], &[0x03, 0x00]); // size LE
        assert_eq!(&chunk[6..], &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn program_command_layout() {
        let cmd = build_program_command(0x1234, 0x97DE, 2);
        assert_eq!(cmd.len(), PROGRAM_HEADER_SIZE);
        assert_eq!(cmd[0], 0x01);
        assert_eq!(cmd[1], b'P');
        assert_eq!(&cmd[2..4], &[0x34, 0x12]); // length LE
        assert_eq!(&cmd[4..6], &[0xDE, 0x97]); // crc16 LE
        assert_eq!(cmd[6], 2); // slot
        assert_eq!(cmd[7], 0x00); // reserved
    }

    #[test]
    fn reload_and_reset_commands() {
        assert_eq!(build_reload_command(), vec![0x01, b'L']);
        assert_eq!(build_reset_command(None), vec![0x01, b'R']);
        assert_eq!(build_reset_command(Some(1)), vec![0x01, b'R', 1]);
    }
}
