# Contributing to Nitrolaunch
Just fork and make a PR, about as simple as that. Try to only work on the `dev` branch, as `main` is for finished releases.

## Project structure
- `/` and `/src`: The root of the project and the `nitrolaunch` crate. This is where most of the library code is for Nitrolaunch's features, such as profiles and configuration. It is split into a handful of large modules that should be pretty self-explanatory.
- `/crates`: Other crates that `nitrolaunch` either uses or is used by.
- `/crates/auth`: Authentication for different types of accounts.
- `/crates/config`: Nitrolaunch config deserialization.
- `/crates/core`: The core launcher library that Nitrolaunch uses.
- `/crates/cli`: The command-line interface for Nitrolaunch.
- `/crates/mods`: Modifications for the core, such as Fabric and Paper.
- `/crates/parse`: Package script parsing.
- `/crates/pkg`: Contains all of the standard formats and utilities for dealing with Nitrolaunch packages. Has the declarative format, dependency resolution, package script evaluation, the repository format, and meta/props evaluation.
- `/crates/pkg_gen`: Generation for Nitrolaunch packages from platforms like Modrinth and Smithed.
- `/crates/plugin`: Allows you to load and use plugins using the Nitrolaunch plugin format. Also provides an API for plugins to use.
- `/crates/shared`: Shared types and utils for all of the Nitrolaunch crates that can't really live anywhere else.
- `/crates/options`: Generation of game options in a backwards-compatible manner.
- `/crates/tools`: A command line utility that uses Nitrolaunch to do certain tasks, mostly relating to generating files.
- `/plugins`: Standard plugins that Nitrolaunch provides
- `/tools`: Some assorted scripts and tools to help development.
