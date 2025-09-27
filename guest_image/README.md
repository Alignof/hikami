# Prepare guest image

## Device tree

Place `guest.dts` to be given to the guest.  
It should require a change in memory layout etc.  
Build is done automatically by cargo build.

## Initrd

Place the symbolic link to the initrd.
It is automatically embedded in the binary.  
Or copy directly to `.guest_initrd` section with a bootloader such as u-boot.

## Linux

Place the symbolic link to the vmlinux.
It is automatically embedded in the binary.

## Example

```sh
$ cd guest_image/
$ ls -l qemu/
.rw-r--r-- 5.5k takana 24 Sep 14:21 cpu0.dtb
.rw-r--r--  11k takana 24 Sep 17:08 cpu0.dts
.rw-r--r-- 5.5k takana 24 Sep 14:21 cpu1.dtb
.rw-r--r--  11k takana 24 Sep 17:08 cpu1.dts
.rw-r--r-- 5.5k takana 24 Sep 14:21 cpu2.dtb
.rw-r--r--  11k takana 24 Sep 17:08 cpu2.dts
.rw-r--r-- 5.5k takana 24 Sep 14:21 cpu3.dtb
.rw-r--r--  11k takana 24 Sep 17:08 cpu3.dts
lrwxrwxrwx    - takana 27 Sep 14:08 vmlinux -> ../../../linux_for_qemu/vmlinux*
```
