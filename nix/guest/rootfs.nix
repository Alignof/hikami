# ext4 root filesystem for the hikami guest, mounted from the AHCI SATA disk.
#
# - interactive (`test = false`): busybox init starts a shell on the console.
# - test (`test = true`): /sbin/init prints a marker and powers off, which ends
#   QEMU through SBI SRST.
{ pkgs
, lib
, busybox
, test ? false
,
}:

let
  bin = "${busybox}/bin";

  inittab = pkgs.writeText "inittab" ''
    ::sysinit:${bin}/mount -t proc proc /proc
    ::sysinit:${bin}/mount -t sysfs sysfs /sys
    ::respawn:-${bin}/sh
    ::shutdown:${bin}/umount -a -r
  '';

  testInit = pkgs.writeTextFile {
    name = "hikami-guest-test-init";
    executable = true;
    text = ''
      #!${bin}/sh
      export PATH=${bin}
      mount -t proc proc /proc
      mount -t sysfs sysfs /sys
      echo "hikami-guest: boot ok"
      uname -a
      echo "hikami-guest: poweroff"
      poweroff -f
    '';
  };

  init = if test then testInit else "${bin}/init";
in
pkgs.callPackage "${pkgs.path}/nixos/lib/make-ext4-fs.nix" {
  storePaths = [ busybox ] ++ lib.optional test testInit;
  volumeLabel = "hikami-root";
  populateImageCommands = ''
    mkdir -p ./files/{sbin,etc,dev,proc,sys,tmp,root}
    ln -s ${bin} ./files/bin
    ln -s ${init} ./files/sbin/init
    ${lib.optionalString (!test) "cp ${inittab} ./files/etc/inittab"}
  '';
}
