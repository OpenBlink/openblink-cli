# OpenBlink CLI

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/OpenBlink/openblink-cli)
[![CI](https://github.com/OpenBlink/openblink-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/OpenBlink/openblink-cli/actions/workflows/ci.yml)
[![Release](https://github.com/OpenBlink/openblink-cli/actions/workflows/release.yml/badge.svg)](https://github.com/OpenBlink/openblink-cli/actions/workflows/release.yml)

A cross-platform command-line tool for [OpenBlink](https://github.com/OpenBlink/openblink)
devices. `openblink-cli` compiles Ruby (mruby) source to bytecode and
**Blink**s it to a device over Bluetooth Low Energy in well under a second — no
rebuild, no reflash, no reboot. The microcontroller never restarts: only the
mruby/c VM is reloaded, keeping the edit-run loop at the speed of thought.

It brings the OpenBlink *Build & Blink* workflow to your terminal and to CI,
speaking the same BLE protocol as the WebIDE and the VS Code extension.

## OpenBlink Ecosystem

OpenBlink is developed as a family of repositories that work together:

- **[openblink](https://github.com/OpenBlink/openblink)** — Core device firmware
- **[openblink-cli](https://github.com/OpenBlink/openblink-cli)** — Command-line *Build & Blink* tool (this repository)
- **[openblink-vscode-extension](https://github.com/OpenBlink/openblink-vscode-extension)** — VS Code extension for editing Ruby and Blinking over BLE
- **[openblink-webide](https://github.com/OpenBlink/openblink-webide)** — Browser-based Web IDE for OpenBlink

## Features

- **Build & Blink** — compile a `.rb` file, stream it to the device over BLE,
  and reLoad the mruby/c VM without restarting the microcontroller.
- **Native mruby compiler** — mruby is vendored and statically linked, so no
  external `mrbc` or WebAssembly runtime is required.
- **Cross-platform** — macOS, Windows, and Linux on both x86_64 and ARM64.
- **Device console** — stream console output from the device over BLE.
- **Offline compile** — produce `.mrb` bytecode without a device.
- **Reset / reLoad** — trigger a full reboot or a VM-only reLoad.

## Supported platforms

| OS | x86_64 | ARM64 |
|----|:------:|:-----:|
| macOS | yes | yes (Apple Silicon) |
| Windows | yes | yes |
| Linux | yes | yes |

## Installation

### Download a release

Prebuilt archives for every supported platform are attached to each
[GitHub Release](https://github.com/OpenBlink/openblink-cli/releases). Download
the archive for your OS/architecture, extract it, and place the `openblink`
binary on your `PATH`.

### Build from source

**Prerequisites**

- [Rust](https://rustup.rs/) 1.83 or newer
- A C compiler — Xcode Command Line Tools (macOS), MSVC Build Tools (Windows),
  or `gcc`/`clang` (Linux)
- Ruby — used to run mruby's `minirake` during the build
- Git — the mruby source is included as a submodule
- Linux only: `libdbus-1-dev` and `pkg-config` for BLE

```sh
git clone --recurse-submodules https://github.com/OpenBlink/openblink-cli
cd openblink-cli
cargo build --release
# the binary is at target/release/openblink
```

If you cloned without `--recurse-submodules`, fetch the mruby source first:

```sh
git submodule update --init --recursive
```

## Quick start

```sh
# 1. Discover nearby devices
openblink scan

# 2. Build a Ruby file and blink it to the first device found
openblink blink examples/blink.rb

# 3. Stream the device console
openblink console
```

## Commands

| Command | Description |
|---------|-------------|
| `scan` | Scan for nearby OpenBlink devices |
| `blink <file>` | Compile, transfer, and reLoad a Ruby file (Build & Blink) |
| `compile <file>` | Compile a Ruby file to `.mrb` without using BLE |
| `console` | Connect to a device and stream console output until `Ctrl-C` |
| `reset` | Send a reset (`R`) command (full reboot, or a single slot) |
| `reload` | Send a reLoad (`L`) command (reLoad the mruby/c VM only) |

### `scan`

```sh
openblink scan [--timeout <seconds>]   # default: 10
```

Scans for the full timeout and lists every OpenBlink device found. All other
BLE commands stop scanning as soon as the target device is discovered, so they
usually connect in well under a second.

### `blink`

```sh
openblink blink <file.rb> [--device <name|address>] [--slot <1|2>]
```

Compiles `<file.rb>`, transfers the bytecode in MTU-sized chunks, sends the
program header (length + CRC16 + slot), and issues a reLoad. The default slot
is `1`.

### `compile`

```sh
openblink compile <file.rb> [--output <path.mrb>]
```

Writes RITE bytecode. When `--output` is omitted, the input path is reused with
a `.mrb` extension. This command does not use Bluetooth.

### `console`

```sh
openblink console [--device <name|address>]
```

### `reset` / `reload`

```sh
openblink reset [--device <name|address>] [--slot <1|2>]
openblink reload [--device <name|address>]
```

`reset` without `--slot` performs a full reboot; with a slot it resets that
slot. `reload` issues a reLoad, recycling the mruby/c VM while keeping the BLE
connection alive.

### Global options

- `--device <name|address>` — select a device by name substring or address.
  When omitted, the first discovered OpenBlink device is used.
- `--timeout <seconds>` — maximum time to scan for devices (default: 10).
  Commands other than `scan` return as soon as the target device is found.
- `-v`, `-vv`, `-vvv` — increase logging verbosity (info, debug, trace).
- `--version` — print the CLI version and the bundled mruby (`mrbc`) version.
- `--help` — print help for the CLI or a subcommand.

## How Blink works

```
Compile (.rb -> .mrb)
  -> Data    (D): bytecode in MTU-sized chunks
  -> Program (P): total length + CRC16 + target slot
  -> reLoad  (L): reload the mruby/c VM
```

Because only the VM layer is recycled by the reLoad (`L`) command, the BLE
connection and console subscription stay active across a Blink.

## BLE protocol

The CLI faithfully implements the OpenBlink BLE protocol:

- **Service** `227da52c-e13a-412b-befb-ba2256bb7fbe`
- **Program characteristic** — `D`/`P`/`L`/`R` (Data, Program, reLoad, Reset) commands and status notifications
- **Console characteristic** — console output notifications
- **MTU characteristic** — negotiated ATT MTU (uint16, little-endian)
- **CRC16** — reflected polynomial `0xD175`, seed `0xFFFF`

See the
[BLE protocol reference](https://github.com/OpenBlink/openblink-vscode-extension/blob/main/doc/ble-protocol.md)
for the full specification.

## Development

```sh
cargo test            # unit tests, including CRC16 golden-vector checks
cargo fmt --all       # format
cargo clippy --all-targets -- -D warnings
```

CI builds and tests on native runners for all six target platforms; releases
are published automatically when a `v*` tag is pushed.

Each release build also compiles a fixed Ruby program and compares the bytecode
against a committed golden (`tests/fixtures/bytecode_golden.mrb`) to catch build
environments that silently miscompile the mruby compiler. After intentionally
updating the mruby submodule, regenerate the golden and commit it:

```sh
UPDATE_GOLDEN=1 cargo test --test bytecode_golden
```

## Platform notes

- **macOS** — the first BLE operation triggers a Bluetooth permission prompt.
  Grant your terminal access under *System Settings → Privacy & Security →
  Bluetooth*.
- **Linux** — requires BlueZ and a running D-Bus/Bluetooth service.
- **Windows** — Bluetooth must be enabled in system settings.

## License

`openblink-cli` is licensed under the [BSD-3-Clause](./LICENSE) license.

It statically links [mruby](https://github.com/mruby/mruby), which is
distributed under the MIT license. The mruby `LICENSE` and `LEGAL` notices are
bundled under [`licenses/`](./licenses) and shipped with every release. See
[`licenses/README.md`](./licenses/README.md) for details.
