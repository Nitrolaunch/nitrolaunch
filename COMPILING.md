# Prerequisites
Make sure you have Rust and Cargo installed (https://rustup.rs).

Set the following variables:

`RUSTUP_TOOLCHAIN=stable`
`CARGO_TARGET_DIR=target`

Fetch all rust dependencies for your architecture using `cargo fetch --locked --target $(rustc --print host-tuple)`.

# Compiling the CLI

To compile the cli, run `cargo build -p nitro_cli --frozen --profile fast_release`.   
The built binary will be located at target/fast_release/nitro.

To run in dev mode, run `cargo run -p nitro_cli -- CLI ARGS HERE`.

To install `nitro` to your system, run `cargo install --path crates/cli --locked`.  
Make sure you have `~/.cargo/bin` in your `PATH` as well.

# Compiling the GUI

The Nitrolaunch GUI uses the [Freya framework](https://freyaui.dev/)

To build the app run `cargo tauri build --no-bundle -- --frozen --profile fast_release`  
Note: To build the debug version append `--all-features` to the above build command.

To test the build, run `cargo test --frozen` and append `--all-features` if the flag was also used in the build stage.

The final static executable will be found at target/fast_release/nitro_gui

# Compiling Plugins
Plugins are built and installed using a Makefile. Some plugins use WASM and need the `wasm32-wasip2` Rust target installed. Install it with `rustup target add wasm32-wasip2`.

`cd` into the `plugins` directory and run `make install.<plugin_name>` to install the plugin you want into your Nitrolaunch plugins directory.


If anything doesn't work here, feel free to ask in the Discord.
