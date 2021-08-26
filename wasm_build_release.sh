RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --out-dir generated --web target/wasm32-unknown-unknown/release/wenderer.wasm
