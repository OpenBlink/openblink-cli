use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::compiler;

pub fn run(file: &Path, output: Option<&Path>) -> Result<()> {
    let source =
        fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))?;
    let filename = file.to_string_lossy();

    let bytecode = compiler::compile_ruby(&source, &filename)
        .with_context(|| format!("failed to compile {}", file.display()))?;

    let out_path = match output {
        Some(p) => p.to_path_buf(),
        None => file.with_extension("mrb"),
    };
    fs::write(&out_path, &bytecode)
        .with_context(|| format!("failed to write {}", out_path.display()))?;

    println!(
        "Compiled {} -> {} ({} bytes)",
        file.display(),
        out_path.display(),
        bytecode.len()
    );
    Ok(())
}
