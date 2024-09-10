#!/bin/bash

function help() {
    echo "create: create dts from qemu dtb"
    echo "build: build dtb from guest.dts"
}

if [ "$#" -eq 0 ]; then
    help
fi

if [ "$#" -eq 1 ]; then
    case "$1" in
        "create")
            qemu-system-riscv64 -S -gdb tcp::10000 \
                -machine virt \
                -bios none  \
                -m 256M \
                -drive file=../rootfs.img,format=raw,id=hd0 \
                -device virtio-blk-device,drive=hd0 \
                -append "root=/dev/vda rw console=ttyS0" \
                -kernel ../target/riscv64imac-unknown-none-elf/debug/hikami \
                -machine dumpdtb=qemu.dtb
            dtc -I dtb -O dts -o guest.dts qemu.dtb
            rm -f qemu.dtb
            ;;
        "build")
            dtc -I dts -O dtb -o ../guest.dtb guest.dts
            ;;
        *)
            echo "command not found"
            help
            ;;
    esac
fi
