{
  lib,
  rustPlatform,
  fetchurl,
  pkg-config,
  patchelf,
  curl,
  openssl,
  fontconfig,
  freetype,
  libxkbcommon,
  wayland,
  libGL,
  vulkan-loader,
  libx11,
  libxcursor,
  libxrandr,
  libxi,
}:
let
  skiaBinaries = fetchurl {
    url = "https://github.com/marc2332/rust-skia/releases/download/0.100.0/skia-binaries-1fd50f8aba4a43c07214-x86_64-unknown-linux-gnu-egl-gl-jpegd-jpege-svg-textlayout-vulkan-wayland-webpd-webpe-x11.tar.gz";
    hash = "sha256-6MwPgtqUWVS35qkp4B8laMOOz4Y3Tg+WSvYs3zDMz8k=";
  };
in
rustPlatform.buildRustPackage {
  pname = "nitrolaunch-gui";
  version = "latest";

  src = ../.;

  buildType = "fast_release";
  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "freya-0.5.0-rc.6" = "sha256-rxY+9T1bvuaZc36AwqLW1iq3yYicCm0jfqMVQ8ycf28=";
    };
  };

  cargoBuildFlags = [ "--package" "nitro_gui" ];

  env.SKIA_BINARIES_URL = "file://${skiaBinaries}";

  nativeBuildInputs = [ pkg-config patchelf curl ];
  buildInputs = [
    openssl
    fontconfig
    freetype
    libxkbcommon
    wayland
    libGL
    vulkan-loader
    libx11
    libxcursor
    libxrandr
    libxi
  ];

  postInstall = ''
    mv $out/bin/nitro_gui $out/bin/Nitrolaunch

    install -Dm644 crates/gui/assets/Nitrolaunch.desktop \
      $out/share/applications/Nitrolaunch.desktop
    sed -i 's|^Exec=.*|Exec=Nitrolaunch|; s|^Icon=.*|Icon=nitrolaunch|' \
      $out/share/applications/Nitrolaunch.desktop
    install -Dm644 crates/gui/assets/icon_128x128.png \
      $out/share/icons/hicolor/128x128/apps/nitrolaunch.png
    install -Dm644 crates/gui/assets/icon_1024x1024.png \
      $out/share/icons/hicolor/1024x1024/apps/nitrolaunch.png
  '';

  postFixup = ''
    patchelf --add-rpath ${lib.makeLibraryPath [
      libGL
      vulkan-loader
      wayland
      libxkbcommon
      libx11
      libxcursor
      libxrandr
      libxi
      fontconfig
    ]} $out/bin/Nitrolaunch
  '';

  meta = with lib; {
    description = "Fast, extensible, and powerful Minecraft launcher";
    homepage = "https://github.com/Nitrolaunch/nitrolaunch";
    license = licenses.gpl3;
    mainProgram = "Nitrolaunch";
  };
}
