#!/bin/zsh

VERSION=0.29.0

mkdir -p release

mkdir -p release/unzip
rm -r release/unzip
mkdir release/unzip

mkdir -p release/out
rm -r release/out
mkdir release/out

unzip -d release/unzip release/macos-latest.zip
unzip -d release/unzip release/windows-2022.zip
unzip -d release/unzip release/ubuntu-latest.zip

mv "release/unzip/universal-apple-darwin/release/bundle/dmg/Nitrolaunch_${VERSION}_universal.dmg" release/out
mv "release/unzip/release/bundle/msi/Nitrolaunch_${VERSION}_x64_en-US.msi" release/out
mv "release/unzip/aarch64-pc-windows-msvc/release/bundle/msi/Nitrolaunch_${VERSION}_arm64_en-US.msi" release/out
mv "release/unzip/release/Nitrolaunch.exe" release/out
mv "release/unzip/aarch64-pc-windows-msvc/release/Nitrolaunch.exe" release/out/Nitrolaunch_arm64.exe

mv "release/unzip/release/bundle/deb/Nitrolaunch_${VERSION}_amd64.deb" release/out
mv "release/unzip/release/bundle/appimage/Nitrolaunch_${VERSION}_amd64.AppImage" release/out

rm -r release/unzip

