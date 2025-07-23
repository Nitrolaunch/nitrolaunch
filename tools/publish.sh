#!/bin/sh
# Run from the root directory of the repository

cd crates

# nitro_shared
cd shared
cargo publish
cd ..

# nitro_auth
cd auth
cargo publish
cd ..

# nitro_net
cd net
cargo publish
cd ..

# nitro_core
cd core
cargo publish
cd ..

# nitro_mods
cd mods
cargo publish
cd ..

# nitro_parse
cd parse
cargo publish
cd ..

# nitro_pkg
cd pkg
cargo publish
cd ..

# nitro_pkg_gen
cd pkg_gen
cargo publish
cd ..

# nitro_config
cd config
cargo publish
cd ..

# nitro_options
cd options
cargo publish
cd ..

# nitro_plugin
cd plugin
cargo publish
cd ..

# nitrolaunch
cd ..
cargo publish

# nitro_cli
cd crates/cli
cargo publish

cd ../..
