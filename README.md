# hikami
[![Rust](https://github.com/Alignof/hikami/actions/workflows/rust.yml/badge.svg)](https://github.com/Alignof/hikami/actions/workflows/rust.yml)  
Light weight type-1 hypervisor for RISC-V H-extension.

This project aims not only to realize a lightweight hypervisor that can be used on RISC-V H extensions, but also to easily reproduce and manage the "extension" on the hypervisor. (currently in progress)  
Poster in RISC-V Days Tokyo 2024 Summer: [PDF](https://riscv.or.jp/wp-content/uploads/RV-Days_Tokyo_2024_Summer_paper_9.pdf)

## Run Linux
### Build QEMU
We need to build manually the QEMU to support IOMMU.  
```
$ git clone https://github.com/qemu/qemu.git -b staging
$ cd qemu/
# https://patchwork.ozlabs.org/project/qemu-devel/list/?series=417654
$ wget https://patchwork.ozlabs.org/series/417654/mbox/ --output-document riscv-QEMU-RISC-V-IOMMU-Support.patch
$ git apply riscv-QEMU-RISC-V-IOMMU-Support.patch
$ ./configure --target-list=riscv64-softmmu
$ make -j $(nproc)
# $ sudo make install
```
Ver. 9.2 or later should officially support IOMMU, so it should no longer be necessary to apply patches.

### Build Linux
```
$ git clone https://github.com/torvalds/linux -b v6.9

$ cd /path/to/this/repository
$ cp ./guest_image/.config /path/to/linux

$ cd /path/to/linux
$ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- defconfig
$ make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- -j$(nproc)
$ mv vmlinux /path/to/this/repository
```
See also for custom guest image: `guest_image/README.md`.

### Create rootfs
```
$ git clone https://gitee.com/mirrors/busyboxsource.git
$ cd busyboxsource

# Select: Settings -> Build Options -> Build static binary
$ CROSS_COMPILE=riscv64-unknown-linux-gnu- make menuconfig

$ CROSS_COMPILE=riscv64-unknown-linux-gnu- make -j8
$ CROSS_COMPILE=riscv64-unknown-linux-gnu- make install

$ cd ../
$ qemu-img create rootfs.img  1g
$ mkfs.ext4 rootfs.img

$ mkdir rootfs
$ mount -o loop rootfs.img rootfs
$ cd rootfs
$ cp -r ../busyboxsource/_install/* .
$ mkdir proc dev tec etc/init.d

$ cd etc/init.d/
$ cat << EOS > rcS
#!/bin/sh
mount -t proc none /proc
mount -t sysfs none /sys
/sbin/mdev -s
EOS

$ chmod +x rcS

$ umount rootfs
$ mv rootfs.img /path/to/this/repository
```

### Run
```
# The actual command to be executed is written in .cargo/config.toml.
$ cargo r
```

## Documents
```
$ cargo doc --open
```

## References
- [The RISC-V Instruction Set Manual: Volume I Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/unpriv-isa-asciidoc.pdf)
- [The RISC-V Instruction Set Manual: Volume II Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/priv-isa-asciidoc.pdf)
- [Rvirt](https://github.com/mit-pdos/RVirt)
- [hypocaust-2](https://github.com/KuangjuX/hypocaust-2)

## Acknowledgement
Exploratory IT Human Resources Project (MITOU Program) of Information-technology Promotion Agency, Japan (IPA) in the fiscal year 2024.  
[https://www.ipa.go.jp/jinzai/mitou/it/2024/gaiyou-tn-3.html](https://www.ipa.go.jp/jinzai/mitou/it/2024/gaiyou-tn-3.html)
