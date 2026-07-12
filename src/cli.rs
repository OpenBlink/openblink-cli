// SPDX-FileCopyrightText: Copyright (c) 2025-2026 OpenBlink All Rights Reserved.
// SPDX-License-Identifier: BSD-3-Clause

//! Command-line argument and subcommand definitions.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::ble::protocol::Slot;

#[derive(Parser, Debug)]
#[command(
    name = "openblink",
    about = "Build & Blink CLI for OpenBlink devices over BLE",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Increase logging verbosity (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Maximum time in seconds to scan for devices.
    #[arg(long, default_value_t = 10, global = true)]
    pub timeout: u64,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scan for nearby OpenBlink devices.
    Scan,

    /// Compile a Ruby file, transfer it to the device, and reLoad in one step.
    Blink {
        /// Ruby source file to build and blink.
        file: PathBuf,
        /// Target device name or address (defaults to the first OpenBlink found).
        #[arg(long)]
        device: Option<String>,
        /// Program slot to write.
        #[arg(long, default_value_t = Slot::One)]
        slot: Slot,
    },

    /// Compile a Ruby file to .mrb without using BLE.
    Compile {
        /// Ruby source file to compile.
        file: PathBuf,
        /// Output path (defaults to the input file with a .mrb extension).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Connect to a device and stream Console output until Ctrl-C.
    Console {
        /// Target device name or address (defaults to the first OpenBlink found).
        #[arg(long)]
        device: Option<String>,
    },

    /// Send a reset (R) command to the device.
    Reset {
        /// Target device name or address (defaults to the first OpenBlink found).
        #[arg(long)]
        device: Option<String>,
        /// Optional slot to reset.
        #[arg(long)]
        slot: Option<Slot>,
    },

    /// Send a reLoad (L) command to the device.
    Reload {
        /// Target device name or address (defaults to the first OpenBlink found).
        #[arg(long)]
        device: Option<String>,
    },
}
