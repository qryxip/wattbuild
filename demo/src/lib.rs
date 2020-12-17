use proc_macro::TokenStream;
use watt::WasmMacro;

static WATT_DEMO: WasmMacro =
    WasmMacro::new(include_bytes!(concat!(env!("OUT_DIR"), "/watt_demo.wasm")));

#[proc_macro_derive(Demo)]
pub fn demo(input: TokenStream) -> TokenStream {
    WATT_DEMO.proc_macro("demo", input)
}
