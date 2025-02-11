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
$ ls -l 
total 28
-rw-r--r-- 1 takana takana 4262 Feb  4 17:01 fpga.dts
-rw-r--r-- 1 takana takana 4865 Feb  4 17:12 guest.dtb
-rw-r--r-- 1 takana takana 5834 Feb  4 17:01 guest.dts
lrwxrwxrwx 1 takana takana   41 Feb  4 17:09 initrd -> ../../vivado-risc-v/debian-riscv64/initrd
-rw-r--r-- 1 takana takana  453 Feb  4 17:27 README.md
lrwxrwxrwx 1 takana takana   40 Feb  4 17:09 vmlinux -> ../../vivado-risc-v/linux-stable/vmlinux
```
