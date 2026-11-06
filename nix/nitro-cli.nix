{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "nitrolaunch-cli";
  version = "latest";

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "freya-0.5.0-rc.6" = "sha256-rxY+9T1bvuaZc36AwqLW1iq3yYicCm0jfqMVQ8ycf28=";
    };
  };
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
