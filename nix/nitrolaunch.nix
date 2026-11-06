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
        alsa-lib
        libjack2
        libpulseaudio
        pipewire

        glfw3-minecraft
        sdl3
        libGL
        vulkan-loader
        wayland
        libdecor
        libx11
        libxcursor
        libxext
        libxrandr
        libxxf86vm

        libxrender
        libxtst
        libxi
        libxcb
        fontconfig
        freetype

        stdenv.cc.cc.lib
        udev 
        libusb1 
        flite 
        gamemode.lib

        mesa-demos
        pciutils 
        xrandr 
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
