# Third-Party Licenses

`openblink-cli` statically links the [mruby](https://github.com/mruby/mruby)
interpreter to compile Ruby source into mruby bytecode (`.mrb`). The mruby
license and its legal notes are reproduced here and are distributed alongside
every released binary.

| Component | License | Files |
|-----------|---------|-------|
| mruby | MIT (with the additional notes in `LEGAL`) | [`mruby/LICENSE`](./mruby/LICENSE), [`mruby/LEGAL`](./mruby/LEGAL) |

The mruby source is pinned as a git submodule under `vendor/mruby`; the files
in this directory are copies kept for distribution so the notices travel with
the compiled binaries.

## Rust dependencies

The remaining dependencies are pulled from crates.io and are licensed under
permissive terms (predominantly `MIT OR Apache-2.0`, with BSD-3-Clause for
`btleplug`). A full machine-readable inventory can be regenerated with
[`cargo-about`](https://github.com/EmbarkStudios/cargo-about) or
[`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny):

```sh
cargo install cargo-about
cargo about generate about.hbs > THIRD_PARTY.html
```
