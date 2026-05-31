# Minimal mruby build configuration for OpenBlink CLI.
#
# It produces a static `libmruby.a` that contains only what is required to
# turn Ruby source into RITE bytecode (`.mrb`):
#   - mruby core (includes the bytecode dumper, `mrb_dump_irep`)
#   - mruby-compiler (parser + code generator)
#
# Presym is disabled so that the C shim can be compiled against the public
# headers in `vendor/mruby/include` alone (no generated `presym/id.h`).
# `MRUBY_BUILD_DIR` is supplied by `build.rs` so artifacts land in Cargo's
# `OUT_DIR` instead of polluting the submodule working tree.
MRuby::Build.new('host') do |conf|
  # `build.rs` sets MRUBY_TOOLCHAIN so the mruby ABI matches the Rust target
  # (e.g. `visualcpp` for *-pc-windows-msvc). Fall back to auto-detection.
  toolchain = ENV['MRUBY_TOOLCHAIN']
  if toolchain && !toolchain.empty?
    conf.toolchain toolchain.to_sym
  else
    conf.toolchain
  end

  # Only the compiler is needed; keep the dependency/license surface minimal.
  conf.gem core: 'mruby-compiler'

  conf.disable_presym
end
