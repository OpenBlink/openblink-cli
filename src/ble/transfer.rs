// SPDX-FileCopyrightText: Copyright (c) 2025 ViXion Inc. All Rights Reserved.
// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! Program (bytecode) transfer sequence: Data chunks -> Program -> reLoad.
//!
//! Ported from `openblink-webide` (`public_html/js/state/ble-transfer.js`).

use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};

use super::crc::crc16;
use super::manager::Connection;
use super::protocol::{self, Slot, MAX_PROGRAM_SIZE};

/// Validates a program against the protocol constraints before transfer.
pub fn validate_program(bytecode: &[u8], mtu: usize) -> Result<()> {
    if bytecode.is_empty() {
        return Err(anyhow!("program is empty (0 bytes)"));
    }
    if bytecode.len() > MAX_PROGRAM_SIZE {
        return Err(anyhow!(
            "program exceeds {MAX_PROGRAM_SIZE} bytes (got {})",
            bytecode.len()
        ));
    }
    if mtu <= protocol::DATA_HEADER_SIZE {
        return Err(anyhow!(
            "negotiated MTU {mtu} is too small (must be greater than {})",
            protocol::DATA_HEADER_SIZE
        ));
    }
    Ok(())
}

/// Transfers `bytecode` to the device and issues the reLoad command.
pub async fn transfer(conn: &Connection, bytecode: &[u8], slot: Slot) -> Result<()> {
    validate_program(bytecode, conn.mtu)?;

    let payload_size = conn.payload_size();
    let total = bytecode.len();

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("  transferring [{bar:30}] {bytes}/{total_bytes}")
            .expect("valid template")
            .progress_chars("=>-"),
    );

    let mut offset = 0usize;
    while offset < total {
        let end = (offset + payload_size).min(total);
        let cmd = protocol::build_data_chunk(offset as u16, &bytecode[offset..end]);
        conn.write_program(&cmd).await?;
        offset = end;
        pb.set_position(offset as u64);
    }
    pb.finish_and_clear();

    let crc = crc16(bytecode);
    let program_cmd = protocol::build_program_command(total as u16, crc, slot);
    conn.write_program(&program_cmd).await?;

    let reload_cmd = protocol::build_reload_command();
    conn.write_program(&reload_cmd).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_rules() {
        assert!(validate_program(b"", 20).is_err()); // empty
        assert!(validate_program(b"abc", 6).is_err()); // mtu too small
        assert!(validate_program(b"abc", 20).is_ok());
    }
}
