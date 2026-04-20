use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use git2::{Repository, Signature};
use serde_json::Value;

use riku::adapters::xschem_driver::XschemDriver;
use riku::core::driver::RikuDriver;

#[derive(Debug)]
struct CmdResult {
    code: i32,
    stdout: String,
    stderr: String,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn run_rust(args: &[&str], cwd: &Path) -> CmdResult {
    let output = Command::new(env!("CARGO_BIN_EXE_riku"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run rust cli");
    CmdResult {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn run_python(args: &[&str], cwd: &Path) -> Option<CmdResult> {
    let code = "from riku.cli import main; main()";
    let output = Command::new("python")
        .current_dir(cwd)
        .env("PYTHONPATH", repo_root())
        .args(["-c", code])
        .args(args)
        .output()
        .ok()?;
    Some(CmdResult {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
        .replace('\\', "/")
        .trim_end()
        .to_string()
}

fn python_available() -> bool {
    Command::new("python")
        .current_dir(repo_root())
        .env("PYTHONPATH", repo_root())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .args(["-c", "import riku, pygit2, typer"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn parity_available() -> bool {
    static PY_AVAILABLE: OnceLock<bool> = OnceLock::new();
    *PY_AVAILABLE.get_or_init(python_available)
}

fn commit_file(repo: &Repository, rel_path: &str, content: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("workdir");
    let full_path = workdir.join(rel_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full_path, content).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(rel_path)).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Riku", "riku@example.com").unwrap();

    let oid = match repo.head() {
        Ok(head) => {
            let parent = repo.find_commit(head.target().unwrap()).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                .unwrap()
        }
        Err(_) => repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
            .unwrap(),
    };
    oid
}

fn commit_empty(repo: &Repository, message: &str) -> git2::Oid {
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Riku", "riku@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
        .unwrap()
}

fn make_repo_with_two_revisions(content_a: &str, content_b: &str) -> (tempfile::TempDir, String) {
    let root = repo_root();
    let temp = tempfile::Builder::new()
        .prefix("riku-parity")
        .tempdir_in(&root)
        .unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    let rel_path = "design/top.sch".to_string();
    commit_file(&repo, &rel_path, content_a, "base");
    commit_file(&repo, &rel_path, content_b, "update");
    (temp, rel_path)
}

fn rust_driver_render(content: &[u8], path_hint: &str) -> Option<PathBuf> {
    let driver = XschemDriver::new();
    driver.render(content, path_hint)
}

fn python_driver_render(sch_path: &Path) -> Option<String> {
    let code = r#"
from pathlib import Path
from riku.adapters.xschem_driver import XschemDriver
sch_path = Path(__import__("os").environ["RIKU_SCH_PATH"])
driver = XschemDriver()
content = sch_path.read_bytes()
out = driver.render(content, str(sch_path))
print("" if out is None else str(out))
"#;
    let output = Command::new("python")
        .current_dir(repo_root())
        .env("PYTHONPATH", repo_root())
        .env("RIKU_SCH_PATH", sch_path)
        .args(["-c", code])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(normalize(&String::from_utf8_lossy(&output.stdout)))
}

fn assert_same_output(rust: CmdResult, python: CmdResult) {
    assert_eq!(rust.code, python.code);
    assert_eq!(normalize(&rust.stdout), normalize(&python.stdout));
    assert_eq!(normalize(&rust.stderr), normalize(&python.stderr));
}

#[test]
fn python_and_rust_diff_match_for_existing_file() {
    if !parity_available() {
        eprintln!("python/parity deps no disponibles; se omite fase 8.");
        return;
    }

    let base = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/SH/op_sim.sch"));
    let updated = base.replace("value=0.9", "value=1.0");
    let (temp, rel_path) = make_repo_with_two_revisions(base, &updated);

    let args = [
        "diff",
        "HEAD~1",
        "HEAD",
        &rel_path,
        "--repo",
        temp.path().to_str().unwrap(),
        "--format",
        "json",
    ];
    let rust = run_rust(&args, repo_root().as_path());
    let python = run_python(&args, repo_root().as_path()).expect("python cli");

    assert_eq!(rust.code, python.code);
    let rust_json: Value = serde_json::from_str(&normalize(&rust.stdout)).unwrap();
    let py_json: Value = serde_json::from_str(&normalize(&python.stdout)).unwrap();
    assert_eq!(rust_json, py_json);
    assert_eq!(normalize(&rust.stderr), normalize(&python.stderr));
}

#[test]
fn python_and_rust_diff_match_for_added_file() {
    if !parity_available() {
        eprintln!("python/parity deps no disponibles; se omite fase 8.");
        return;
    }

    let root = repo_root();
    let temp = tempfile::Builder::new()
        .prefix("riku-parity-added")
        .tempdir_in(&root)
        .unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    commit_empty(&repo, "base");
    let rel_path = "design/top.sch";
    let content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/SH/op_sim.sch"));
    commit_file(&repo, rel_path, content, "add");

    let args = [
        "diff",
        "HEAD~1",
        "HEAD",
        rel_path,
        "--repo",
        temp.path().to_str().unwrap(),
        "--format",
        "text",
    ];
    let rust = run_rust(&args, repo_root().as_path());
    let python = run_python(&args, repo_root().as_path()).expect("python cli");

    assert_same_output(rust, python);
}

#[test]
fn python_and_rust_log_match() {
    if !parity_available() {
        eprintln!("python/parity deps no disponibles; se omite fase 8.");
        return;
    }

    let base = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/SH/op_sim.sch"));
    let updated = base.replace("value=0.9", "value=1.0");
    let (temp, rel_path) = make_repo_with_two_revisions(base, &updated);

    let args = [
        "log",
        &rel_path,
        "--repo",
        temp.path().to_str().unwrap(),
        "--limit",
        "10",
        "--semantic",
    ];
    let rust = run_rust(&args, repo_root().as_path());
    let python = run_python(&args, repo_root().as_path()).expect("python cli");

    assert_same_output(rust, python);
}

#[test]
fn python_and_rust_doctor_match() {
    if !parity_available() {
        eprintln!("python/parity deps no disponibles; se omite fase 8.");
        return;
    }

    let root = repo_root();
    let temp = tempfile::Builder::new()
        .prefix("riku-parity-doctor")
        .tempdir_in(&root)
        .unwrap();
    let rust = run_rust(&["doctor", "--repo", temp.path().to_str().unwrap()], repo_root().as_path());
    let python = run_python(&["doctor", "--repo", temp.path().to_str().unwrap()], repo_root().as_path()).expect("python cli");

    assert_same_output(rust, python);
}

#[test]
fn python_and_rust_render_match() {
    if !parity_available() {
        eprintln!("python/parity deps no disponibles; se omite fase 8.");
        return;
    }

    let sch_path = repo_root().join("examples").join("SH").join("op_sim.sch");
    let content = fs::read(&sch_path).unwrap();
    let rust = rust_driver_render(&content, sch_path.to_str().unwrap());
    let python = python_driver_render(&sch_path);

    let rust_norm = rust.as_ref().map(|p| normalize(&p.to_string_lossy()));
    assert_eq!(rust_norm, python);

    if let Some(path) = rust {
        assert!(path.exists());
        let svg = fs::read_to_string(&path).unwrap();
        assert!(svg.contains("</svg>"));
    }
}
