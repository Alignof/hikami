{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      fenix,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-MFHoWVcPZKjFrLnQnodCXbUsfZcofGfo9tV9fTTvklg=";
        };
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        formatter = pkgs.nixpkgs-fmt;
        # Tools required to build hikami (used as is in CI)
        devShells.ci = pkgs.mkShell {
          name = "hikami-ci";
          nativeBuildInputs = [
            toolchain
            pkgs.dtc
          ];
        };
        devShells.default = pkgs.mkShell {
          name = "rust environment";
          inputsFrom = [ self.devShells.${system}.ci ];
          nativeBuildInputs = [
            pkgs.tokei
            pkgs.qemu
          ];
        };
      }
    );
}
