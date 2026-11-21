use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // exec `cargo metadata` and get a json.
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("Failed to execute cargo metadata");

    let metadata = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // parse the json.
    let json: serde_json::Value = serde_json::from_str(&metadata).expect("Failed to parse JSON");

    // retrieve `workspace_root`.
    let workspace_root = json.get("workspace_root").and_then(|v| v.as_str()).unwrap();

    // extract `features` in `macro_crate`.
    let root_manifest = Path::new(workspace_root).join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", root_manifest.display());

    let root_crate = json
        .get("packages")
        .and_then(|v| v.as_array())
        .and_then(|packages| {
            packages.iter().find(|pkg| {
                pkg.get("manifest_path")
                    .and_then(|p| p.as_str())
                    .is_some_and(|p| Path::new(p) == root_manifest)
            })
        })
        .expect("Failed to find root_crate package");

    // collect crates regardless of `enable_extension`.
    let feature_dependencies: HashSet<String> = root_crate
        .get("features")
        .and_then(|v| v.as_object())
        .and_then(|features| features.get("enable_extension")) // crates regardless of `enable_extension`
        .and_then(|v| v.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|dep| dep.as_str())
                .filter(|dep| !dep.contains('/'))
                .map(|dep| dep.replace('-', "_")) // replace `-` with `_`
                .collect()
        })
        .unwrap_or_default();

    // Extension crates are only linked when the root crate enables `enable_extension`.
    let crate_names = if env::var_os("CARGO_FEATURE_ENABLE_EXTENSION").is_some() {
        feature_dependencies.into_iter().collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    // output crates list to `OUT_DIR/dependencies.rs`
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = format!("{out_dir}/dependencies.rs");
    let content = format!("static CRATES: &[&str] = &{crate_names:?};");

    fs::write(out_path, content).expect("Failed to write dependencies.rs");
}
