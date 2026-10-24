//! Workspace tasks.
//!
//! The only task that cannot be expressed as a plain `cargo` invocation is
//! plugin bundling: NIH-plug's own bundler copies the built `cdylib` into the
//! CLAP/VST3 bundle layout a host expects. Delegating to it instead of
//! reimplementing keeps the bundle format in sync with the plugin framework.
//!
//! ```text
//! cargo xtask bundle peak-plugin --release   # -> target/bundled/
//! ```

fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()
}
