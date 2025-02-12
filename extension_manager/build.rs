use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // `cargo metadata` を実行して JSON を取得
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("Failed to execute cargo metadata");

    let metadata = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // JSON をパース
    let json: serde_json::Value = serde_json::from_str(&metadata).expect("Failed to parse JSON");

    // `workspace_root` を取得
    let workspace_root = json.get("workspace_root").and_then(|v| v.as_str()).unwrap();
    let root_package_name = Path::new(workspace_root)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap();

    // `macro_crate` の `features` を取得
    let root_crate = json
        .get("packages")
        .and_then(|v| v.as_array())
        .and_then(|packages| {
            packages
                .iter()
                .find(|pkg| pkg.get("name").and_then(|n| n.as_str()) == Some(&root_package_name))
        })
        .expect("Failed to find root_crate package");

    dbg!(&root_crate);

    // `enable_extension` に関連するクレートを取得
    let feature_dependencies: HashSet<String> = root_crate
        .get("features")
        .and_then(|v| v.as_object())
        .and_then(|features| features.get("enable_extension")) // `enable_extension` に紐づいたクレート
        .and_then(|v| v.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|dep| dep.as_str())
                .map(|dep| dep.replace('-', "_")) // `-` → `_` に変換
                .collect()
        })
        .unwrap_or_default();
    let crate_names = feature_dependencies.into_iter().collect::<Vec<_>>();

    // 取得したクレート一覧を `OUT_DIR/dependencies.rs` に出力
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = format!("{}/dependencies.rs", out_dir);
    let content = format!("static CRATES: &[&str] = &{:?};", crate_names);

    fs::write(out_path, content).expect("Failed to write dependencies.rs");
}
