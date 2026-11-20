//! build.rs - For build script for cargo project.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

/// Build script for cargo project
#[allow(clippy::similar_names)]
fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // --- Device Tree Compilation ---
    let guest_image_dir = PathBuf::from("guest_image");
    let target_dirs = ["qemu", "megrez"];

    for target_dir_name in target_dirs {
        let target_path = guest_image_dir.join(target_dir_name);
        println!("cargo:rerun-if-changed={}", target_path.display());

        for entry in WalkDir::new(&target_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "dts"))
        {
            let dts_file = entry.path();
            let dtb_file = dts_file.with_extension("dtb");

            println!(
                "Compiling DT: {} -> {}",
                dts_file.display(),
                dtb_file.display()
            );

            let status = Command::new("dtc")
                .args(["-I", "dts", "-O", "dtb", "-o"])
                .arg(&dtb_file)
                .arg(dts_file)
                .status()
                .expect("Failed to execute dtc");

            assert!(
                status.success(),
                "dtc failed for {} with exit status: {}",
                dts_file.display(),
                status
            );

            // Tell cargo to rerun the build script if the DTS file changes.
            println!("cargo:rerun-if-changed={}", dts_file.display());
        }
    }
    // --- End Device Tree Compilation ---

    // --- Linker Script ---
    // `link.x` includes `memory.x`, which is the memory layout of the target board.
    let board = if env::var_os("CARGO_FEATURE_QEMU").is_some() {
        "qemu"
    } else if env::var_os("CARGO_FEATURE_MEGREZ").is_some() {
        "megrez"
    } else {
        // `target_board` reports the missing board feature.
        return;
    };
    let memory_x = PathBuf::from("linker_scripts").join(format!("{board}.x"));
    fs::copy(&memory_x, out_dir.join("memory.x")).unwrap();
    fs::copy("link.x", out_dir.join("link.x")).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rerun-if-changed={}", memory_x.display());
    println!("cargo:rerun-if-changed=link.x");
    // --- End Linker Script ---

    println!("cargo:rerun-if-changed=build.rs");
}
