# Minimal Linux kernel for the hikami guest on QEMU virt.
#
# Only `vmlinux` (ELF) is built because hikami loads the ELF segments
# of the guest kernel. No modules are built.
{ lib
, stdenv
, buildPackages
, linux_6_16
,
}:

stdenv.mkDerivation {
  pname = "hikami-guest-linux";
  inherit (linux_6_16) version src;

  depsBuildBuild = [ buildPackages.stdenv.cc ];
  nativeBuildInputs = with buildPackages; [
    bc
    bison
    flex
    perl
    python3
  ];

  makeFlags = [
    "ARCH=riscv"
    "CROSS_COMPILE=${stdenv.cc.targetPrefix}"
    "HOSTCC=${buildPackages.stdenv.cc.targetPrefix}cc"
    "KBUILD_BUILD_VERSION=1"
    "KBUILD_BUILD_USER=hikami"
    "KBUILD_BUILD_HOST=nix"
  ];

  # Reproducible build: `makeFlags` is split on spaces, so set it here.
  env.KBUILD_BUILD_TIMESTAMP = "Thu Jan 1 00:00:00 UTC 1970";

  postPatch = ''
    patchShebangs scripts
  '';

  configurePhase = ''
    runHook preConfigure
    make $makeFlags defconfig
    scripts/kconfig/merge_config.sh -m .config ${./linux.config}
    make $makeFlags olddefconfig
    runHook postConfigure
  '';

  buildPhase = ''
    runHook preBuild
    make $makeFlags -j$NIX_BUILD_CORES vmlinux
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    install -Dm644 vmlinux $out/vmlinux
    install -Dm644 .config $out/config
    runHook postInstall
  '';

  meta.platforms = lib.platforms.riscv64;
}
