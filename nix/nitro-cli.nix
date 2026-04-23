{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "nitrolaunch-cli";
  version = "latest";

  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;
  buildType = "fast_release";

  cargoBuildFlags = [
    "--package"
    "nitro_cli"
  ];

  meta = with lib; {
    description = "Fast, extensible, and powerful Minecraft launcher";
    homepage = "https://github.com/Nitrolaunch/nitrolaunch";
    license = licenses.gpl3;
    mainProgram = "nitro";
  };
})
