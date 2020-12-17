fn main() {
    wattbuild::build(
        &[r#"{ package = "watt-demo", git = "https://github.com/dtolnay/watt" }"#],
        true,
        None,
        None,
    );
}
