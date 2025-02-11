# hikami
[![Rust](https://github.com/Alignof/hikami/actions/workflows/rust.yml/badge.svg)](https://github.com/Alignof/hikami/actions/workflows/rust.yml)  
A lightweight Type-1 hypervisor for RISC-V H-extension, featuring **RISC-V extension emulation**.

This project aims not only to realize a lightweight hypervisor that can be used on RISC-V H extensions, but also to easily reproduce and manage the "extension" on the hypervisor. 
Poster in RISC-V Days Tokyo 2024 Summer: [PDF](https://riscv.or.jp/wp-content/uploads/RV-Days_Tokyo_2024_Summer_paper_9.pdf)  
Paper in ComSys2024(ja): [link](https://ipsj.ixsq.nii.ac.jp/records/241051)

## Prepare
```sh
$ git clone https://github.com/buildroot/buildroot.git
$ cd buildroot/
$ make qemu_riscv64_virt_defconfig
$ make -j$(nproc)
$ ln -s output/images/rootfs.ext2 path/to/hikami/rootfs.ext2
$ ln -s output/build/linux-x.x.x/vmlinux path/to/hikami/guest_image/vmlinux
# optional
$ ln -s path/to/initrd path/to/hikami/guest_image/initrd

# copy host dts and edit to change user memory config
# QEMU's dtb can be obtained by adding the option `-machine dumpdtb=qemu.dtb`.
$ vim guest_image/guest.dts
```

## Run on QEMU
```sh
# The actual command to be executed is written in .cargo/config.toml.
$ cargo r
```

## Run on FPGA
The target FPGAs are as the following. (boards supported by vivado-riscv repository)
```
- AMD VC707 
- AMD KC705 
- Digilent Genesys 2 
- Digilent Nexys Video 
- Digilent Nexys A7 100T 
- Digilent Arty A7 100T
```

### Prepare the FPGA
```sh
# set environment
$ git clone https://github.com/Alignof/vivado-risc-v -b feature/hikami
$ cd vivado-risc-v
$ make update-submodules

# Build FPGA bitstream
# Connect a micro-B cable to `PROG`
$ source /opt/Xilinx/Vivado/2024.2/settings64.sh
$ make CONFIG=rocket64b1 BOARD=nexys-video bitstream

# Prepare the SD card
$ ./mk-sd-card

# Program the FPGA flash memory
$ Xilinx/Vivado/2023.2/bin/hw_server
$ env HW_SERVER_URL=tcp:localhost:3121 xsdb -quiet board/jtag-freq.tcl
$ make CONFIG=rocket64b2 BOARD=nexys-video flash
```

See also for an environment information: [https://github.com/Alignof/vivado-risc-v/blob/master/README.md](https://github.com/Alignof/vivado-risc-v/blob/master/README.md)

### Run
```sh
# Connect a micro-B cable to `UART`
$ sudo picocom -b 115200 /dev/ttyUSB2 # <- select the corresponding serial port 

# login: debian
# password: debian
```

## Documents
```sh
$ cargo doc --open
```

## Related projects
- [ozora](https://github.com/Alignof/ozora): Generat0r for hypervisor(hikami) module and decoder (raki). 
- [raki](https://github.com/Alignof/raki): RISC-V instruction decoder.
- [wild-screen-alloc](https://github.com/Alignof/wild-screen-alloc): Slab allocator for bare-metal Rust.

## References
- [The RISC-V Instruction Set Manual: Volume I Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/unpriv-isa-asciidoc.pdf)
- [The RISC-V Instruction Set Manual: Volume II Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/priv-isa-asciidoc.pdf)
- [Rvirt](https://github.com/mit-pdos/RVirt)
- [hypocaust-2](https://github.com/KuangjuX/hypocaust-2)

## Acknowledgement
Exploratory IT Human Resources Project (MITOU Program) of Information-technology Promotion Agency, Japan (IPA) in the fiscal year 2024.  
[https://www.ipa.go.jp/jinzai/mitou/it/2024/gaiyou-tn-3.html](https://www.ipa.go.jp/jinzai/mitou/it/2024/gaiyou-tn-3.html)
