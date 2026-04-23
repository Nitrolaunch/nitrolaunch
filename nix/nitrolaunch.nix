{
  symlinkJoin,
  buildFHSEnv,
  callPackage,
  extraPkgs ? pkgs: [ ],
}:
let
  nitrolaunch-unwrapped = callPackage ./nitrolaunch-unwrapped.nix {};
  nitro-cli-unwrapped = callPackage ./nitro-cli.nix {};
  fhsEnv = {
    version = "latest";
    targetPkgs =
      pkgs:
      with pkgs;
      [
        openal
        glfw3-minecraft
        alsa-lib
        libjack2
        libpulseaudio
        pipewire
        libGL
        libx11
        libxcursor
        libxext
        libxrandr
        libxxf86vm
        vulkan-loader
      ]
      ++ [nitro-cli-unwrapped nitrolaunch-unwrapped]
      ++ extraPkgs pkgs;
  };
in
symlinkJoin {
  name = "nitrolaunch";
  paths = [
    (buildFHSEnv (
      fhsEnv
      // {
        pname = "Nitrolaunch";
        runScript = "Nitrolaunch";
      }
    ))
    (buildFHSEnv (
      fhsEnv
      // {
        pname = "nitro";
        runScript = "nitro";
      }
    ))
  ];
  postBuild = ''
    mkdir -p $out/share
    ln -s ${nitrolaunch-unwrapped}/share/applications $out/share
    ln -s ${nitrolaunch-unwrapped}/share/icons $out/share
  '';

  inherit (nitrolaunch-unwrapped) meta;
}
