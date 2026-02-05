#!/bin/zsh

VERSION=0.29.0

mkdir -p unzip
rm -r unzip
mkdir unzip

mkdir -p out
rm -r out
mkdir out

unzip -d unzip macos-latest.zip
unzip -d unzip windows-2022.zip
unzip -d unzip ubuntu-latest.zip

mv "unzip/universal-apple-darwin/release/bundle/dmg/Nitrolaunch_${VERSION}_universal.dmg" out
mv "unzip/release/bundle/msi/Nitrolaunch_${VERSION}_x64_en-US.msi" out
mv "unzip/aarch64-pc-windows-msvc/release/bundle/msi/Nitrolaunch_${VERSION}_arm64_en-US.msi" out
mv "unzip/release/Nitrolaunch.exe" out
mv "unzip/aarch64-pc-windows-msvc/release/Nitrolaunch.exe" out/Nitrolaunch_arm64.exe

mv "unzip/release/bundle/deb/Nitrolaunch_${VERSION}_amd64.deb" out
mv "unzip/release/bundle/appimage/Nitrolaunch_${VERSION}_amd64.AppImage" out

rm -r unzip

