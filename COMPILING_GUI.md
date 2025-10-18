# Compiling the GUI

The Nitrolaunch GUI uses [Tauri](https://v1.tauri.app/).

## 1. Prerequisites

Follow the [Tauri guide](https://v1.tauri.app/v1/guides/getting-started/prerequisites) to install the necessary software for your system, including system libraries, NodeJS, and Rust.

## 2. Install Tauri CLI

Install npm if you haven't already, then `cd` into the `gui` directory and run `npm install --save-dev @tauri-apps/cli@"1.6.3"` in your terminal. Note that you might have to use a different npm package depending on what system you are on.

## 3. Build / Debug

To debug, simply run `npm run tauri dev` inside the `gui` directory. To build release bundles of the app for the current system, run `npm run tauri build`, and the bundles should end up somewhere under `target/release/bundle`.


If anything doesn't work here, feel free to ask in the Discord.
