//! Builds WASI runtimes in `build.rs` for `watt`.
//!
//! # Usage
//!
//! ```toml
//! [package]
//! name = "watt_demo_automated"
//! version = "0.1.0"
//! edition = "2018"
//!
//! [lib]
//! proc-macro = true
//!
//! [dependencies]
//! watt = "0.4.0"
//!
//! [build-dependencies]
//! wattbuild = { git = "https://github.com/qryxip/wattbuild" }
//! ```
//!
//! In `build.rs`:
//!
//! ```no_run
//! fn main() {
//!     wattbuild::build(
//!         &[r#"{ package = "watt-demo", git = "https://github.com/dtolnay/watt" }"#],
//!         None,
//!         None,
//!     );
//! }
//! ```
//!
//! And in `lib.rs`:
//!
//! ```ignore
//! use proc_macro::TokenStream;
//! use watt::WasmMacro;
//!
//! static WATT_DEMO: WasmMacro =
//!     WasmMacro::new(include_bytes!(concat!(env!("OUT_DIR"), "/watt_demo.wasm")));
//!
//! #[proc_macro_derive(Demo)]
//! pub fn demo(input: TokenStream) -> TokenStream {
//!     WATT_DEMO.proc_macro("demo", input)
//! }
//! ```
//!
//! # Working directory
//!
//! The target crates are built under:
//!
//! | Platform | Working Directory                                                      |
//! | :-       | :-                                                                     |
//! | Linux    | `$XDG_CACHE_DIR/wattbuild` or `$HOME/.cache/wattbuild`                 |
//! | macOS    | `$HOME/Library/Caches/wattbuild`                                       |
//! | Windows  | `%APPDATA%\Local\wattbuild` or `%USERPROFILE%\AppData\Local\wattbuild` |
//!
//! # Python
//!
//! This crate requires Python 3.6+ in the `$PATH`.

#![allow(clippy::needless_doctest_main)]

use std::{
    path::Path,
    process::{self, Command},
};

/// Compiles packages in `wasm32-unknown-unknown` and moves the `.wasm` artifacts to `$OUT_DIR`.
///
/// `dependencies` are [dependency specification](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)s with `package` keys.
///
/// `python_exe` defaults to `"python"` on Windows, `"python3"` on other platforms.
///
/// # Example
///
/// ```no_run
/// fn main() {
///     wattbuild::build(
///         &[r#"{ package = "watt-demo", git = "https://github.com/dtolnay/watt" }"#],
///         Some("de066c43e8352c9f187a075f83a7d62ddf91c422"),
///         Some("/usr/bin/python3".as_ref()),
///     );
/// }
/// ```
pub fn build(dependencies: &[&str], proc_macro2_rev: Option<&str>, python_exe: Option<&Path>) -> ! {
    if let Err(err) = run(dependencies, proc_macro2_rev, python_exe) {
        let mut errs = err.into_iter();
        if let Some(err) = errs.next() {
            eprintln!("Error: {}", err);
            for err in errs {
                eprintln!("Caused by: {}", err);
            }
        }
        process::exit(1);
    }
    process::exit(0);
}

fn run(
    dependencies: &[&str],
    proc_macro2_rev: Option<&str>,
    python_exe: Option<&Path>,
) -> Result<(), Vec<String>> {
    let python_exe =
        python_exe.unwrap_or_else(|| (if cfg!(windows) { "python" } else { "python3" }).as_ref());

    let mut args = dependencies.to_owned();
    if let Some(proc_macro2_rev) = proc_macro2_rev {
        args.push("--proc-macro2-rev");
        args.push(proc_macro2_rev);
    }

    let status = Command::new(python_exe)
        .args(&["-c", include_str!("./wattbuild.py")])
        .args(args)
        .status()
        .map_err(|err| {
            vec![
                format!("could not execute `{}`", python_exe.display()),
                err.to_string(),
            ]
        })?;

    if !status.success() {
        return Err(vec![format!(
            "`{}` failed: {}",
            python_exe.display(),
            status,
        )]);
    }
    Ok(())
}
