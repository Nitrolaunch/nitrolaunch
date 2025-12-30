# Prerequisites
Make sure you have Rust and Cargo installed (https://rustup.rs).

# Compiling the CLI

To run in dev mode, run `cargo run -p nitro_cli -- CLI ARGS HERE`.

To install `nitro` to your system, run `cargo install --path crates/cli --locked`. Make sure you have `~/.cargo/bin` in your `PATH` as well.

# Compiling the GUI

The Nitrolaunch GUI uses [Tauri](https://v1.tauri.app/).

## 1. Prerequisites

Follow the [Tauri guide](https://v1.tauri.app/v1/guides/getting-started/prerequisites) to install the necessary software for your system, including system libraries, NodeJS, and Rust.

## 2. Install Tauri CLI

Install npm if you haven't already, then `cd` into the `gui` directory and run `cargo install tauri-cli --version "^2.9.3" --locked` in your terminal (you can also use `cargo binstall` if you have it).

## 3. Build / Debug

First run `npm install` to install dependencies. To debug, simply run `cargo tauri dev` inside the `gui` directory. To build release bundles of the app for the current system, run `cargo tauri tauri build`, and the bundles should end up somewhere under `target/release/bundle`.


If anything doesn't work here, feel free to ask in the Discord.

# Compiling Plugins
Plugins are built and installed using a Makefile. Some plugins use WASM and need the `wasm32-wasip2` Rust target installed. Install it with `rustup target add wasm32-wasip2`.

`cd` into the `plugins` directory and run `make install.<plugin_name>` to install the plugin you want into your Nitrolaunch plugins directory.
