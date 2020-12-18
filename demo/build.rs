use wattbuild::{Dependency, Source};

fn main() {
    wattbuild::build(
        &[Dependency {
            package: "watt-demo",
            source: Source::Git {
                git: "https://github.com/dtolnay/watt",
                rev: None,
            },
        }],
        None,
        None,
    );
}
