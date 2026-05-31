//! Release-gate test: verifies that the bundled mruby compiler (`mrbc`) emits
//! the exact RITE bytecode we expect.
//!
//! Some build environments have been observed to miscompile `mrbc` so that it
//! produces invalid bytecode *without* reporting an error. RITE output is
//! deterministic and platform-independent for a fixed mruby version: integers
//! are serialized big-endian, the compiler name/version fields are constant,
//! and no timestamps are embedded. A byte-for-byte comparison against a
//! committed golden file therefore detects such a faulty toolchain on every
//! target the release workflow builds.
//!
//! Regenerate the golden after an intentional mruby update with:
//!   UPDATE_GOLDEN=1 cargo test --test bytecode_golden

use std::path::PathBuf;
use std::process::Command;

/// mruby version the committed golden was produced with. Bumping the mruby
/// submodule changes the bytecode, so this guard fails loudly and points at the
/// regeneration command.
const EXPECTED_MRUBY_VERSION: &str = "mruby 3.4.0";

const REGEN_HINT: &str = "UPDATE_GOLDEN=1 cargo test --test bytecode_golden";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bytecode_golden.rb")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bytecode_golden.mrb")
}

/// Compiles the fixture with the freshly built `openblink` binary and returns
/// the emitted `.mrb` bytes.
fn compile_fixture() -> Vec<u8> {
    let out_path =
        std::env::temp_dir().join(format!("openblink_golden_{}.mrb", std::process::id()));
    let _ = std::fs::remove_file(&out_path);

    let status = Command::new(env!("CARGO_BIN_EXE_openblink"))
        .arg("compile")
        .arg(fixture_path())
        .arg("--output")
        .arg(&out_path)
        .status()
        .expect("failed to run the openblink binary");
    assert!(status.success(), "`openblink compile` failed: {status}");

    let bytes = std::fs::read(&out_path).expect("failed to read the compiled .mrb output");
    let _ = std::fs::remove_file(&out_path);
    bytes
}

/// Returns the `openblink --version` output (includes the linked mruby version).
fn version_output() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_openblink"))
        .arg("--version")
        .output()
        .expect("failed to run `openblink --version`");
    assert!(output.status.success(), "`openblink --version` failed");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Returns the offset of the first differing byte, or where the shorter buffer
/// ends when one is a prefix of the other.
fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    if let Some(pos) = a.iter().zip(b.iter()).position(|(x, y)| x != y) {
        Some(pos)
    } else if a.len() == b.len() {
        None
    } else {
        Some(a.len().min(b.len()))
    }
}

#[test]
fn mrbc_emits_expected_bytecode() {
    let bytes = compile_fixture();

    // Regeneration mode: rewrite the golden and skip the comparison.
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(golden_path(), &bytes).expect("failed to write the golden file");
        eprintln!(
            "updated golden: {} ({} bytes)",
            golden_path().display(),
            bytes.len()
        );
        return;
    }

    // Structural sanity checks give a clearer message than a raw byte diff when
    // the output is grossly malformed. Layout per mruby/dump.h:
    //   [0..4] "RITE"  [4..8] format version "0300"  [12..16] compiler "MATZ".
    assert!(
        bytes.len() >= 20,
        "output is too small to be a RITE binary ({} bytes)",
        bytes.len()
    );
    assert_eq!(&bytes[0..4], b"RITE", "missing RITE magic");
    assert_eq!(&bytes[4..8], b"0300", "unexpected RITE format version");
    assert_eq!(&bytes[12..16], b"MATZ", "unexpected RITE compiler name");

    // Tie the golden to a known mruby version so an intentional submodule bump
    // fails here with an actionable hint instead of an opaque byte mismatch.
    let version = version_output();
    assert!(
        version.contains(EXPECTED_MRUBY_VERSION),
        "`--version` does not contain {EXPECTED_MRUBY_VERSION:?}:\n{version}\n\
         If the mruby submodule was updated intentionally, regenerate the golden:\n  {REGEN_HINT}"
    );

    let golden = std::fs::read(golden_path()).unwrap_or_else(|e| {
        panic!(
            "failed to read golden {}: {e}\nGenerate it with:\n  {REGEN_HINT}",
            golden_path().display()
        )
    });

    assert!(
        bytes == golden,
        "mrbc produced unexpected bytecode.\n\
         golden: {} bytes\nactual: {} bytes\nfirst difference at byte: {:?}\n\n\
         This usually means the build toolchain miscompiled mruby. If instead the mruby \
         submodule was updated intentionally, regenerate the golden:\n  {REGEN_HINT}",
        golden.len(),
        bytes.len(),
        first_difference(&bytes, &golden),
    );
}
