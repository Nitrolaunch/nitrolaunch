# Prerequisites

Make sure you have Rust and Cargo installed (https://rustup.rs).

The GUI might need some additional deps on linux like fontconfig.

# Compiling the CLI

To compile the cli, run `cargo build -p nitro_cli --locked --profile fast_release`.  
The built binary will be located at target/fast_release/nitro.

To run in dev mode, run `cargo run -p nitro_cli -- CLI ARGS HERE`.

To install `nitro` to your system, run `cargo install --path crates/cli --locked`.  
Make sure you have `~/.cargo/bin` in your `PATH` as well.

# Compiling the GUI

To build the app run `cargo build -p nitro_gui --profile fast_release`  
Note: To build the debug version remove `--profile fast_release` from the above build command.

To test the build, run `cargo test`

The final static executable will be found at target/fast_release/nitro_gui

# Compiling Plugins

Plugins are built and installed using a Makefile. Some plugins use WASM and need the `wasm32-wasip2` Rust target installed. Install it with `rustup target add wasm32-wasip2`.

`cd` into the `plugins` directory and run `make install.<plugin_name>` to install the plugin you want into your Nitrolaunch plugins directory.

If anything doesn't work here, feel free to ask in the Discord.
