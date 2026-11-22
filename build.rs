use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mruby_root = manifest_dir.join("vendor").join("mruby");
    let shim = manifest_dir.join("csrc").join("openblink_mrbc.c");
    let build_config = manifest_dir.join("build_config").join("openblink.rb");

    println!("cargo:rerun-if-changed={}", shim.display());
    println!("cargo:rerun-if-changed={}", build_config.display());
    println!(
        "cargo:rerun-if-changed={}",
        mruby_root
            .join("include")
            .join("mruby")
            .join("version.h")
            .display()
    );
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RUBY");

    // Ensure the mruby submodule has been checked out.
    if !mruby_root.join("include").join("mruby.h").exists() {
        panic!(
            "mruby submodule not found at {}.\n\
             Run: git submodule update --init --recursive",
            mruby_root.display()
        );
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_msvc = target_env == "msvc";

    // Build a static mruby library (compiler + bytecode dumper) into Cargo's
    // OUT_DIR using the vendored `minirake` (only requires a Ruby interpreter).
    let mruby_build_dir = out_dir.join("mruby");

    // The mruby build dir persists in OUT_DIR across builds. minirake only
    // recompiles objects whose sources changed, so after bumping the vendored
    // mruby the archive would silently mix objects compiled from two different
    // mruby versions. Wipe it whenever the vendored source revision changes.
    let version_header = mruby_root.join("include").join("mruby").join("version.h");
    let mruby_rev = Command::new("git")
        .arg("-C")
        .arg(&mruby_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let source_stamp = format!(
        "{mruby_rev}:{}",
        std::fs::read_to_string(&version_header).unwrap_or_default()
    );
    let stamp_file = mruby_build_dir.join(".source-stamp");
    if std::fs::read_to_string(&stamp_file).ok().as_deref() != Some(source_stamp.as_str()) {
        let _ = std::fs::remove_dir_all(&mruby_build_dir);
    }

    let ruby = env::var("RUBY").unwrap_or_else(|_| "ruby".to_string());
    let mut command = Command::new(&ruby);
    command
        .arg("minirake")
        .current_dir(&mruby_root)
        .env("MRUBY_CONFIG", &build_config)
        .env("MRUBY_BUILD_DIR", &mruby_build_dir);
    // Pin the mruby toolchain to match the Rust target ABI on Windows.
    if target_os == "windows" {
        command.env("MRUBY_TOOLCHAIN", if is_msvc { "visualcpp" } else { "gcc" });
    }
    let status = command.status().unwrap_or_else(|e| {
        panic!("failed to run `{ruby} minirake` (is Ruby installed and on PATH?): {e}")
    });
    if !status.success() {
        panic!("mruby build failed: `{ruby} minirake` exited with {status}");
    }

    std::fs::create_dir_all(&mruby_build_dir).expect("create mruby build dir");
    std::fs::write(&stamp_file, &source_stamp).expect("write mruby source stamp");

    // The MSVC toolchain emits `libmruby.lib`; every other toolchain emits
    // `libmruby.a`.
    let lib_dir = mruby_build_dir.join("host").join("lib");
    let produced = lib_dir.join(if is_msvc {
        "libmruby.lib"
    } else {
        "libmruby.a"
    });
    if !produced.exists() {
        panic!(
            "expected static library was not produced: {}",
            produced.display()
        );
    }

    // rustc resolves `static=mruby` to `mruby.lib` on MSVC, but mruby names the
    // archive `libmruby.lib`; provide a copy under the expected name.
    if is_msvc {
        let expected = lib_dir.join("mruby.lib");
        std::fs::copy(&produced, &expected).unwrap_or_else(|e| {
            panic!(
                "failed to copy {} to {}: {e}",
                produced.display(),
                expected.display()
            )
        });
    }

    // Compile the FFI shim. Since mruby 4.0 presym is always enabled, so the
    // shim must see the generated `mruby/presym/*.h` headers that minirake
    // writes into the build dir's include/.
    cc::Build::new()
        .file(&shim)
        .include(mruby_root.join("include"))
        .include(mruby_build_dir.join("host").join("include"))
        .warnings(false)
        .compile("openblink_mrbc");

    // Link the static mruby library (after the shim that depends on it).
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=mruby");

    // mruby needs the math library on Unix-like targets.
    if target_os != "windows" {
        println!("cargo:rustc-link-lib=dylib=m");
    }
}
