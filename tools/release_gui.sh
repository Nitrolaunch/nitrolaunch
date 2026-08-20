VERSION=0.31.0

mkdir -p release

mkdir -p release/unzip
rm -r release/unzip
mkdir release/unzip

mkdir -p release/out
rm -r release/out
mkdir release/out

unzip -d release/unzip release/linux.zip
unzip -d release/unzip release/windows.zip
unzip -d release/unzip release/macos.zip

# Linux
mv "release/unzip/crates/gui/dist/nitro_gui_${VERSION}_x86_64.AppImage" "release/out/Nitrolaunch_app_${VERSION}_x86_64.AppImage"
mv "release/unzip/crates/gui/dist/nitro_gui_${VERSION}_amd64.deb" "release/out/Nitrolaunch_app_${VERSION}_amd64.deb"
mv "release/unzip/dist/Nitrolaunch.flatpak" "release/out/Nitrolaunch_app_${VERSION}.flatpak"

# Windows
mv "release/unzip/crates/gui/dist/nitro_gui_${VERSION}_x64_en-US.msi" "release/out/Nitrolaunch_app_${VERSION}_x64_en-US.msi"

# MacOS
mv "release/unzip/crates/gui/dist/Nitrolaunch_${VERSION}_aarch64.dmg" "release/out/Nitrolaunch_app_${VERSION}_universal.dmg"

rm -r release/unzip

