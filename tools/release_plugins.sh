mkdir -p release
mkdir -p release/plugins

mkdir -p release/plugins/out
rm -r release/plugins/out
mkdir release/plugins/out

unzip -d release/plugins/out release/plugins/aarch64-apple-darwin.zip
unzip -d release/plugins/out release/plugins/x86_64-apple-darwin.zip
unzip -d release/plugins/out release/plugins/x86_64-pc-windows-gnu.zip
unzip -d release/plugins/out release/plugins/x86_64-unknown-linux-gnu.zip
unzip -d release/plugins/out release/plugins/universal.zip