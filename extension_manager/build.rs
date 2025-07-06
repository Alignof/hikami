use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");

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
    let root_package_name = Path::new(workspace_root)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap();

    // extract `features` in `macro_crate`.
    let root_crate = json
        .get("packages")
        .and_then(|v| v.as_array())
        .and_then(|packages| {
            packages
                .iter()
                .find(|pkg| pkg.get("name").and_then(|n| n.as_str()) == Some(root_package_name))
        })
        .expect("Failed to find root_crate package");

    dbg!(&root_crate);

    // collect crates reguardless of `enable_extension`.
    let feature_dependencies: HashSet<String> = root_crate
        .get("features")
        .and_then(|v| v.as_object())
        .and_then(|features| features.get("enable_extension")) // crates reguardless of `enable_extension`
        .and_then(|v| v.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|dep| dep.as_str())
                .map(|dep| dep.replace('-', "_")) // replace `-` with `_`
                .collect()
        })
        .unwrap_or_default();
    let crate_names = feature_dependencies.into_iter().collect::<Vec<_>>();

    // output crates list to `OUT_DIR/dependencies.rs`
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = format!("{out_dir}/dependencies.rs");
    let content = format!("static CRATES: &[&str] = &{crate_names:?};");

    fs::write(out_path, content).expect("Failed to write dependencies.rs");
}
