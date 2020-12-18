# wattbuild

[![CI](https://github.com/qryxip/wattbuild/workflows/CI/badge.svg)](https://github.com/qryxip/wattbuild/actions?workflow=CI)
[![dependency status](https://deps.rs/repo/github/qryxip/wattbuild/status.svg)](https://deps.rs/repo/github/qryxip/wattbuild)

Builds WASI runtimes in `build.rs` for [`watt`](https://crates.io/crates/watt).

See the docs.rs documentation (not yet) and [the demo](https://github.com/qryxip/wattbuild/tree/master/demo).

```rust
use wattbuild::{Dependency, Source};

fn main() {
    wattbuild::build(
        &[Dependency {
            package: "watt-demo",
            source: Source::Path {
                path: "/path/to/watt/demo/impl",
                or: Some(Box::new(Source::Git {
                    git: "https://github.com/dtolnay/watt",
                    rev: None,
                })),
            },
        }],
        None,
        None,
    );
}
```

## License

Licensed under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).
