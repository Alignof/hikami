# Build guest image

## device tree
```sh
$ ./build_dtb.sh create
$ vim guest.dts # edit dts
$ ./build_dtb.sh build
# guest.dtb is created to repository root.
```

## Linux (with debug info)
```sh
$ git clone https://github.com/torvalds/linux -b v6.9

$ cd /path/to/this/repository
$ cp ./guest_image/.config /path/to/linux

$ cd /path/to/linux
$ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- defconfig
$ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- menuconfig
$ DEBUG_KERNEL [=y], DEBUG_INFO [=y], EFI [=n], RELOCATABLE [=n]
$ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- -j$(nproc)
$ mv vmlinux /path/to/linux/vmlinx_debug
```

## Linux (For Zicfiss)
See [https://lwn.net/Articles/992578/](https://lwn.net/Articles/992578/).
```sh
# Toolchain
$ git clone git@github.com:sifive/riscv-gnu-toolchain.git -b cfi-dev
$ riscv-gnu-toolchain/configure --prefix=<path-to-where-to-build> --with-arch=rv64gc_zicfilp_zicfiss --enable-linux --disable-gdb  --with-extra-multilib-test="rv64gc_zicfilp_zicfiss-lp64d:-static"
$ make -j$(nproc)

# Opensbi
$ git clone git@github.com:deepak0414/opensbi.git -b v6_cfi_spec_split_opensbi
$ make CROSS_COMPILE=<your riscv toolchain> -j$(nproc) PLATFORM=generic

# Linux
$ git clone https://github.com/torvalds/linux -b v6.12-rc1
$ wget https://patchwork.kernel.org/series/896898/mbox/ --output-document riscv-control-flow-integrity-for-usermode.patch
$ git am riscv-control-flow-integrity-for-usermode.patch 
$ make ARCH=riscv CROSS_COMPILE=<path-to-cfi-riscv-gnu-toolchain>/build/bin/riscv64-unknown-linux-gnu- -j$(nproc) defconfig
$ make ARCH=riscv CROSS_COMPILE=<path-to-cfi-riscv-gnu-toolchain>/build/bin/riscv64-unknown-linux-gnu- -j$(nproc)
```
