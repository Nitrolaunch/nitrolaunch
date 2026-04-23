{
  lib,
  rustPlatform,
  pkg-config,
  openssl,
  libsoup_3,
  gtk3,
  webkitgtk_4_1,
  cargo-tauri,
  glib-networking,
  nodejs,
  npmHooks,
  fetchNpmDeps,
  makeWrapper,
  wrapGAppsHook3,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "nitrolaunch-gui";
  version = "latest";

  src = ../.;

  # Needed as nitro_gui is in a cargo workspace
  postPatch = ''
    cp Cargo.lock gui/src-tauri/Cargo.lock

    substituteInPlace gui/src-tauri/tauri.conf.json \
       --replace-fail '"createUpdaterArtifacts": "v1Compatible"' '"createUpdaterArtifacts": false'
  '';

  buildType = "fast_release";
  cargoLock.lockFile = ../Cargo.lock;

  npmDeps = fetchNpmDeps {
    name = "${finalAttrs.pname}-${finalAttrs.version}-npm-deps";
    src = "${finalAttrs.src}/${finalAttrs.npmRoot}";
    lockFile = "package-lock.json";
    hash = "sha256-vszIlNiarQUKL+NX9NaekSSumbVyrWXPpBYZdFgf2XU=";
  };

  npmRoot = "gui";

  nativeBuildInputs = [
    cargo-tauri.hook
    nodejs
    npmHooks.npmConfigHook
    pkg-config
    wrapGAppsHook3
    makeWrapper
  ];

  buildInputs = [
    glib-networking
    openssl
    libsoup_3
    gtk3
    webkitgtk_4_1
  ];

  cargoRoot = "gui/src-tauri";
  buildAndTestSubdir = "gui/src-tauri";

  meta = with lib; {
    description = "Fast, extensible, and powerful Minecraft launcher";
    homepage = "https://github.com/Nitrolaunch/nitrolaunch";
    license = licenses.gpl3;
    mainProgram = "Nitrolaunch";
  };
})
