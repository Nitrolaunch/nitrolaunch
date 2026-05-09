curl -L -o release/ubuntu-latest.zip https://nightly.link/Nitrolaunch/nitrolaunch/workflows/build_gui/main/ubuntu-latest.zip
curl -L -o release/windows-2022.zip https://nightly.link/Nitrolaunch/nitrolaunch/workflows/build_gui/main/windows-2022.zip
curl -L -o release/macos-latest.zip https://nightly.link/Nitrolaunch/nitrolaunch/workflows/build_gui/main/macos-latest.zip

mkdir -p release/out

curl -L -o release/out/cli-windows.zip https://nightly.link/Nitrolaunch/nitrolaunch/workflows/build/main/windows-latest.zip
curl -L -o release/out/cli-macos.zip https://nightly.link/Nitrolaunch/nitrolaunch/workflows/build/main/macos-latest.zip
curl -L -o release/out/cli-linux.zip https://nightly.link/Nitrolaunch/nitrolaunch/workflows/build/main/ubuntu-latest.zip