use framesmith_fspack::PackView;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const CLI_DOC: &str = include_str!("../../docs/cli.md");
const README: &str = include_str!("../../README.md");
const ZX_FSPACK_DOC: &str = include_str!("../../docs/zx-fspack.md");
const AGENTS_DOC: &str = include_str!("../../AGENTS.md");

fn cli_bin() -> PathBuf {
    option_env!("CARGO_BIN_EXE_framesmith-cli")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("debug")
                .join(format!("framesmith-cli{}", std::env::consts::EXE_SUFFIX))
        })
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has repo parent")
        .to_path_buf()
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create destination directory");
    for entry in fs::read_dir(src).expect("read source directory") {
        let entry = entry.expect("read directory entry");
        let ty = entry.file_type().expect("read entry type");
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to);
        } else {
            fs::copy(&from, &to).expect("copy file");
        }
    }
}

fn temp_project() -> TempDir {
    let temp = tempfile::tempdir().expect("create temp project");
    let root = temp.path();
    let repo = repo_root();

    fs::create_dir_all(root.join("src-tauri")).expect("create temp src-tauri");
    fs::copy(
        repo.join("framesmith.rules.json"),
        root.join("framesmith.rules.json"),
    )
    .expect("copy project rules");
    copy_dir_all(&repo.join("characters"), &root.join("characters"));

    let globals = repo.join("globals");
    if globals.exists() {
        copy_dir_all(&globals, &root.join("globals"));
    }

    temp
}

fn run_doc_example(cwd: &Path, args: &[&str]) {
    let output = Command::new(cli_bin())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run framesmith-cli doc example");

    assert!(
        output.status.success(),
        "framesmith-cli failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_valid_fspk(path: &Path) {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let pack =
        PackView::parse(&bytes).unwrap_or_else(|e| panic!("parse {}: {:?}", path.display(), e));
    let states = pack.states().expect("FSPK has states section");
    assert!(!states.is_empty(), "FSPK should contain at least one state");
}

#[test]
fn cli_reference_export_examples_run_against_temp_project() {
    assert!(CLI_DOC.contains(
        "cargo run --bin framesmith-cli -- export --project .. --character test_char --out ../exports/test_char.fspk"
    ));
    assert!(CLI_DOC.contains(
        "cargo run --bin framesmith-cli -- export --project .. --all --out-dir ../exports"
    ));
    assert!(CLI_DOC.contains(
        "cargo run --bin framesmith-cli -- export --characters-dir ../characters --all --out-dir ../exports"
    ));

    let temp = temp_project();
    let cwd = temp.path().join("src-tauri");

    run_doc_example(
        &cwd,
        &[
            "export",
            "--project",
            "..",
            "--character",
            "test_char",
            "--out",
            "../exports/test_char.fspk",
        ],
    );
    assert_valid_fspk(&temp.path().join("exports/test_char.fspk"));

    run_doc_example(
        &cwd,
        &[
            "export",
            "--project",
            "..",
            "--all",
            "--out-dir",
            "../exports",
        ],
    );
    assert_valid_fspk(&temp.path().join("exports/test_char.fspk"));

    run_doc_example(
        &cwd,
        &[
            "export",
            "--characters-dir",
            "../characters",
            "--all",
            "--out-dir",
            "../exports",
        ],
    );
    assert_valid_fspk(&temp.path().join("exports/test_char.fspk"));
}

#[test]
fn readme_and_format_docs_cli_examples_match_the_tested_export_command() {
    let tested_command =
        "cargo run --bin framesmith-cli -- export --project .. --all --out-dir ../exports";
    let tested_single_command = "cargo run --bin framesmith-cli -- export --project .. --character test_char --out ../exports/test_char.fspk";

    assert!(README.contains(tested_command));
    assert!(AGENTS_DOC.contains(tested_command));
    assert!(ZX_FSPACK_DOC.contains(tested_single_command));
    assert!(!AGENTS_DOC.contains("cargo run --bin framesmith -- export"));
}
