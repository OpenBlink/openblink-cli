use std::ffi::{c_char, c_int, c_void, CStr, CString};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("input source is empty")]
    EmptyInput,
    #[error("{0}")]
    Mruby(String),
}

extern "C" {
    fn openblink_mrbc_compile(
        src: *const c_char,
        src_len: usize,
        filename: *const c_char,
        flags: u8,
        out_buf: *mut *mut u8,
        out_len: *mut usize,
        err_msg: *mut *mut c_char,
    ) -> c_int;
    fn openblink_mrbc_free(ptr: *mut c_void);
    fn openblink_mrbc_version() -> *const c_char;
}

const OPENBLINK_MRBC_NO_EXT_OPS: u8 = 0x01;
const OPENBLINK_MRBC_NO_OPTIMIZE: u8 = 0x02;

/// Optional mruby compiler switches, mirroring `mrbc` flags added in mruby 4.0.
///
/// The local-variable section is always stripped from the output (equivalent
/// to `mrbc --remove-lv`) since mruby/c ignores it and it only inflates the
/// BLE transfer payload.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompileOptions {
    /// Disallow OP_EXT1/2/3 extended opcodes (`mrbc --no-ext-ops`). Use for
    /// firmware whose mruby/c was built without `MRBC_SUPPORT_OP_EXT`.
    pub no_ext_ops: bool,
    /// Disable peephole optimizations (`mrbc --no-optimize`), mainly useful
    /// for diagnosing codegen issues.
    pub no_optimize: bool,
}

impl CompileOptions {
    fn flags(self) -> u8 {
        let mut flags = 0;
        if self.no_ext_ops {
            flags |= OPENBLINK_MRBC_NO_EXT_OPS;
        }
        if self.no_optimize {
            flags |= OPENBLINK_MRBC_NO_OPTIMIZE;
        }
        flags
    }
}

/// Returns the version of the statically linked mruby compiler, e.g. `mruby 4.0.0`.
pub fn mrbc_version() -> String {
    // SAFETY: the shim returns a pointer to a static, NUL-terminated string.
    unsafe {
        let ptr = openblink_mrbc_version();
        if ptr.is_null() {
            "mruby (unknown)".to_string()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}

/// Compiles Ruby `source` into RITE bytecode (`.mrb`).
///
/// `filename` is only used to render `file:line:col: message` diagnostics.
pub fn compile_ruby(
    source: &str,
    filename: &str,
    options: CompileOptions,
) -> Result<Vec<u8>, CompileError> {
    if source.trim().is_empty() {
        return Err(CompileError::EmptyInput);
    }

    let c_filename =
        CString::new(filename).unwrap_or_else(|_| CString::new("(input)").expect("valid cstring"));

    let mut out_buf: *mut u8 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut err_msg: *mut c_char = std::ptr::null_mut();

    // SAFETY: we pass valid pointers/lengths; the source need not be NUL
    // terminated because the length is provided explicitly.
    let rc = unsafe {
        openblink_mrbc_compile(
            source.as_ptr() as *const c_char,
            source.len(),
            c_filename.as_ptr(),
            options.flags(),
            &mut out_buf,
            &mut out_len,
            &mut err_msg,
        )
    };

    if rc != 0 {
        // SAFETY: on failure the shim sets err_msg to a malloc'd C string (or null).
        let message = unsafe {
            if err_msg.is_null() {
                "unknown mruby compile error".to_string()
            } else {
                let m = CStr::from_ptr(err_msg).to_string_lossy().into_owned();
                openblink_mrbc_free(err_msg as *mut c_void);
                m
            }
        };
        return Err(CompileError::Mruby(message));
    }

    // SAFETY: on success the shim sets out_buf/out_len to a malloc'd buffer.
    let bytes = unsafe {
        if out_buf.is_null() {
            Vec::new()
        } else {
            let v = std::slice::from_raw_parts(out_buf, out_len).to_vec();
            openblink_mrbc_free(out_buf as *mut c_void);
            v
        }
    };

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_valid_program() {
        let bytes =
            compile_ruby("1 + 1\n", "test.rb", CompileOptions::default()).expect("should compile");
        assert!(bytes.starts_with(b"RITE"), "output must be a RITE binary");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn compiles_with_no_ext_ops_and_no_optimize() {
        let options = CompileOptions {
            no_ext_ops: true,
            no_optimize: true,
        };
        let bytes = compile_ruby(
            "def f(n)\n  n.times { |i| i * 2 }\nend\nf(300)\n",
            "test.rb",
            options,
        )
        .expect("should compile");
        assert!(bytes.starts_with(b"RITE"), "output must be a RITE binary");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(matches!(
            compile_ruby("   \n", "test.rb", CompileOptions::default()),
            Err(CompileError::EmptyInput)
        ));
    }

    #[test]
    fn reports_syntax_error_with_location() {
        let err = compile_ruby("def foo\n", "bad.rb", CompileOptions::default()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad.rb"),
            "error should mention the filename: {msg}"
        );
    }

    #[test]
    fn version_is_mruby_4() {
        assert!(
            mrbc_version().starts_with("mruby 4."),
            "got: {}",
            mrbc_version()
        );
    }
}
