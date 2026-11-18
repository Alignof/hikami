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
    { self
    , nixpkgs
    , utils
    , fenix
    ,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-MFHoWVcPZKjFrLnQnodCXbUsfZcofGfo9tV9fTTvklg=";
        };
        pkgs = nixpkgs.legacyPackages.${system};
        pkgsRiscv = pkgs.pkgsCross.riscv64;
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        # Guest images for running hikami on QEMU
        packages = {
          guest-linux = pkgsRiscv.callPackage ./nix/guest/linux.nix { };
          guest-rootfs = pkgs.callPackage ./nix/guest/rootfs.nix { inherit (pkgsRiscv) busybox; };
          guest-rootfs-test = pkgs.callPackage ./nix/guest/rootfs.nix {
            inherit (pkgsRiscv) busybox;
            test = true;
          };
        };

        # Build hikami and run lint
        devShells.lint = pkgs.mkShell {
          name = "hikami-lint";
          nativeBuildInputs = [
            toolchain
            pkgs.dtc
            pkgs.cargo-make
          ];
        };
        # Additionally run hikami on QEMU
        devShells.test = pkgs.mkShell {
          name = "hikami-test";
          inputsFrom = [ self.devShells.${system}.lint ];
          nativeBuildInputs = [ pkgs.qemu ];
        };
        devShells.default = pkgs.mkShell {
          name = "rust environment";
          inputsFrom = [ self.devShells.${system}.test ];
          nativeBuildInputs = [ pkgs.tokei ];
        };
      }
    );
}
