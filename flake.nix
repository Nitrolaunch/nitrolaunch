{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  outputs =
    inputs:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems =
        set:
        builtins.listToAttrs (
          map (system: {
            name = system;
            value = set inputs.nixpkgs.legacyPackages.${system} system;
          }) systems
        );
    in
    {
      packages = forAllSystems (
        pkgs: _: {
          default = pkgs.callPackage ./nix/nitrolaunch.nix {};
        }
      );
    };
}
