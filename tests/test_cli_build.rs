use std::path::Path;
use std::process::{Command, Output};

fn build(root: &Path, no_cluster: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_graphify-rs"));
    command
        .args([
            "--jobs",
            "1",
            "build",
            "--code-only",
            "--no-llm",
            "--format",
            "json",
        ])
        .arg("--path")
        .arg(root);
    if no_cluster {
        command.arg("--no-cluster");
    }
    command.output().expect("run real build command")
}

fn checked_build(root: &Path, no_cluster: bool, skips: bool) {
    let output = build(root, no_cluster);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).contains("skipping rebuild"),
        skips
    );
    let graph: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("graphify-rs-out/graph.json")).unwrap())
            .unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    assert!(!nodes.is_empty(), "fixture must create graph nodes");
    assert_eq!(
        nodes.iter().any(|node| !node["community"].is_null()),
        !no_cluster
    );
}

#[test]
fn cluster_mode_changes_rebuild_unchanged_sources() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("main.rs"),
        "fn main() { helper(); } fn helper() {}",
    )
    .unwrap();
    checked_build(root.path(), false, false);
    checked_build(root.path(), true, false);
    checked_build(root.path(), true, true);
    checked_build(root.path(), false, false);
    checked_build(root.path(), false, true);

    let mode = root.path().join("graphify-rs-out/cluster-mode");
    std::fs::write(&mode, "invalid previous mode").unwrap();
    checked_build(root.path(), false, false);

    let graph_path = root.path().join("graphify-rs-out/graph.json");
    std::fs::remove_file(&graph_path).unwrap();
    std::fs::create_dir(&graph_path).unwrap();
    assert!(!build(root.path(), true).status.success());
    assert!(
        !mode.exists(),
        "failed export must not preserve successful build mode"
    );
    std::fs::remove_dir(&graph_path).unwrap();
    checked_build(root.path(), true, false);
}

#[test]
fn oversized_source_is_reported_and_excluded_before_caching() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::File::create(root.path().join("oversized.rs"))
        .unwrap()
        .set_len(graphify_core::MAX_SOURCE_BYTES + 1)
        .unwrap();
    let output = build(root.path(), true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let warning = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        warning.contains("skipping source over 3 MiB") && warning.contains("oversized.rs"),
        "missing visible oversized-source warning: {warning}"
    );
    let graph: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.path().join("graphify-rs-out/graph.json")).unwrap(),
    )
    .unwrap();
    assert!(graph["nodes"].as_array().unwrap().iter().all(|node| {
        !node["source_file"]
            .as_str()
            .unwrap()
            .contains("oversized.rs")
    }));
}
