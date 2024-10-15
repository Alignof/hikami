#!/bin/bash

qemu_path="../../qemu_iommu/build/qemu-system-riscv64"

function help() {
    echo "create [host/guest/all]: create host or guest dts from qemu dtb"
    echo "build [host/guest/all]: build host or guest dtb from guest.dts"
}

function create_host() {
    $qemu_path -S -gdb tcp::10000 \
        -machine virt \
        -bios none  \
        -m 256M \
        -initrd ../vmlinux_debug \
        -drive file=../rootfs.ext2,format=raw,id=hd0,if=none \
        -device virtio-blk-device,drive=hd0 \
        -netdev user,id=n1 \
        -device virtio-net-pci,netdev=n1 \
        -device riscv-iommu-pci \
        -append "root=/dev/vda rw console=ttyS0" \
        -kernel ../target/riscv64imac-unknown-none-elf/debug/hikami \
        -machine dumpdtb=qemu.dtb
    dtc -I dtb -O dts -o host.dts qemu.dtb
    rm -f qemu.dtb
}

function create_guest() {
    $qemu_path -S -gdb tcp::10000 \
        -machine virt \
        -bios none  \
        -m 256M \
        -initrd ../vmlinux_debug \
        -drive file=../rootfs.ext2,format=raw,id=hd0,if=none \
        -device virtio-blk-device,drive=hd0 \
        -netdev user,id=n1 \
        -device virtio-net-pci,netdev=n1 \
        -append "root=/dev/vda rw console=ttyS0" \
        -kernel ../target/riscv64imac-unknown-none-elf/debug/hikami \
        -machine dumpdtb=qemu.dtb
    dtc -I dtb -O dts -o guest.dts qemu.dtb
    rm -f qemu.dtb
}

if [ "$#" -eq 0 ]; then
    help
fi

if [ "$#" -eq 1 ]; then
    echo "specify target: host or guest or all"
    help
fi

if [ "$#" -eq 2 ]; then
    case "$1" in
        "create")
            case "$2" in
                "host")
                    create_host
                    ;;
                "guest")
                    create_guest
                    ;;
                "all")
                    create_host
                    create_guest
                    ;;
                *)
                    echo "specify target: host or guest or all"
                    help
                    ;;
            esac
            ;;
        "build")
            case "$2" in
                "host")
                    dtc -I dts -O dtb -o ../host.dtb host.dts
                    ;;
                "guest")
                    dtc -I dts -O dtb -o ../guest.dtb guest.dts
                    ;;
                "all")
                    dtc -I dts -O dtb -o ../host.dtb host.dts
                    dtc -I dts -O dtb -o ../guest.dtb guest.dts
                    ;;
                *)
                    echo "specify target: host or guest or all"
                    help
                    ;;
            esac
            ;;
        *)
            echo "command not found"
            help
            ;;
    esac
fi
