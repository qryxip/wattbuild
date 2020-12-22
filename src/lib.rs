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
//! use wattbuild::{Dependency, Source};
//!
//! fn main() {
//!     wattbuild::build(
//!         &[Dependency {
//!             package: "watt-demo",
//!             source: Source::Path {
//!                 path: "/path/to/watt/demo/impl",
//!                 or: Some(Box::new(Source::Git {
//!                     git: "https://github.com/dtolnay/watt",
//!                     rev: None,
//!                 })),
//!             },
//!         }],
//!         None,
//!         None,
//!         None,
//!     );
//! }
//! ```
//!
//! And in `lib.rs`:
//!
//! ```
//! # extern crate proc_macro;
//! # macro_rules! include_bytes(($($_tt:tt)*) => (b""));
//! use proc_macro::TokenStream;
//! use watt::WasmMacro;
//!
//! static WATT_DEMO: WasmMacro =
//!     WasmMacro::new(include_bytes!(concat!(env!("OUT_DIR"), "/watt_demo.wasm")));
//!
//! # const _: &str = stringify! {
//! #[proc_macro_derive(Demo)]
//! pub fn demo(input: TokenStream) -> TokenStream {
//!     WATT_DEMO.proc_macro("demo", input)
//! }
//! # };
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
    env,
    path::Path,
    process::{self, Command},
};

/// Compiles packages in `wasm32-unknown-unknown` and moves the `.wasm` artifacts to `$OUT_DIR`.
///
/// # Example
///
/// ```no_run
/// use wattbuild::{Dependency, Source};
///
/// fn main() {
///     wattbuild::build(
///         &[Dependency {
///             package: "watt-demo",
///             source: Source::Path {
///                 path: "/path/to/watt/demo/impl",
///                 or: Some(Box::new(Source::Git {
///                     git: "https://github.com/dtolnay/watt",
///                     rev: None,
///                 })),
///             },
///         }],
///         Some("de066c43e8352c9f187a075f83a7d62ddf91c422"),
///         Some("stable"),
///         Some("/usr/bin/python3".as_ref()),
///     );
/// }
/// ```
pub fn build(
    dependencies: &[Dependency],
    proc_macro2_rev: Option<&str>,
    toolchain: Option<&str>,
    python_exe: Option<&Path>,
) -> ! {
    if let Err(err) = run(dependencies, proc_macro2_rev, toolchain, python_exe) {
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
    dependencies: &[Dependency],
    proc_macro2_rev: Option<&str>,
    toolchain: Option<&str>,
    python_exe: Option<&Path>,
) -> Result<(), Vec<String>> {
    let python_exe =
        python_exe.unwrap_or_else(|| (if cfg!(windows) { "python" } else { "python3" }).as_ref());

    let mut args = dependencies
        .iter()
        .map(|d| d.to_specification())
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(toolchain) = toolchain {
        args.push("--toolchain".to_owned());
        args.push(toolchain.to_owned());
    }
    if let Some(proc_macro2_rev) = proc_macro2_rev {
        args.push("--proc-macro2-rev".to_owned());
        args.push(proc_macro2_rev.to_owned());
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

#[derive(Debug)]
pub struct Dependency {
    pub package: &'static str,
    pub source: Source,
}

#[derive(Debug)]
pub enum Source {
    Path {
        path: &'static str,
        or: Option<Box<Self>>,
    },
    Git {
        git: &'static str,
        rev: Option<&'static str>,
    },
    Registry {
        version: &'static str,
        registry: Option<&'static str>,
    },
}

impl Dependency {
    fn to_specification(&self) -> Result<String, Vec<String>> {
        let key_values = self.source.to_key_values()?;

        Ok(format!(
            "{{ package = {}, {} }}",
            to_lit(self.package),
            key_values,
        ))
    }
}

impl Source {
    fn to_key_values(&self) -> Result<String, Vec<String>> {
        match self {
            Source::Path { path, or } => {
                let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|e| {
                    vec![
                        "could not read `$CARGO_MANIFEST_DIR`".to_owned(),
                        e.to_string(),
                    ]
                })?;

                let path = Path::new(&manifest_dir).join(path);
                let path = path
                    .to_str()
                    .ok_or_else(|| vec![format!("non UTF-8 path: `{}`", path.display())])?;

                if Path::new(path).exists() {
                    Ok(format!("path = {}", to_lit(path)))
                } else {
                    or.as_ref()
                        .ok_or_else(|| {
                            vec![format!(
                                "`{}` does not exist and missing `Source::Path::or`",
                                path,
                            )]
                        })?
                        .to_key_values()
                }
            }
            Source::Git { git, rev } => Ok(format!(
                "git = {}{}",
                to_lit(git),
                rev.map(|rev| format!(", rev = {}", to_lit(rev)))
                    .unwrap_or_default(),
            )),
            Source::Registry { version, registry } => Ok(format!(
                "version = {}{}",
                to_lit(version),
                registry
                    .map(|registry| format!(", registry = {}", to_lit(registry)))
                    .unwrap_or_default(),
            )),
        }
    }
}

fn to_lit(s: &str) -> String {
    let mut ret = "\"".to_owned();
    for c in s.chars() {
        match c {
            '"' => ret += "\\\"",
            '\\' => ret += "\\\\",
            '\x08' => ret += "\\b",
            '\x0C' => ret += "\\f",
            '\n' => ret += "\\n",
            '\r' => ret += "\\r",
            '\t' => ret += "\\t",
            c if c != ' ' && (c.is_whitespace() || c.is_control()) => {
                ret += &format!("\\U{:x<06}", c);
            }
            c => ret.push(c),
        }
    }
    ret.push('"');
    ret
}
