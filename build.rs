//! build.rs - For build script for cargo project.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Build script for cargo project
#[allow(clippy::similar_names)]
fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let dts_file = "guest_image/guest.dts";
    let dtb_file = "guest_image/guest.dtb";

    let status = Command::new("dtc")
        .args(["-I", "dts", "-O", "dtb", "-o", dtb_file, dts_file])
        .status()
        .expect("Failed to execute dtc");

    assert!(status.success(), "dtc failed with exit status: {status}");

    // Put the linker script somewhere the linker can find it.
    fs::write(out_dir.join("memory.x"), include_bytes!("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory.x");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={dts_file}");
    println!("cargo:rerun-if-changed={dtb_file}");
}
