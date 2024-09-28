# Build guest image

## device tree
```
$ ./build_dtb.sh create
$ vim guest.dts # edit dts
$ ./build_dtb.sh build
```

## Linux
```
$ git clone https://github.com/torvalds/linux -b v6.9

$ cd /path/to/this/repository
$ cp ./.config /path/to/linux

$ cd /path/to/linux
$ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- defconfig
# For debug
# $ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- menuconfig
# $ DEBUG_KERNEL [=y], DEBUG_INFO [=y], EFI [=n], RELOCATABLE [=n]
$ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- -j$(nproc)
# $ mv vmlinux /path/to/linux/vmlinx_debug
```
