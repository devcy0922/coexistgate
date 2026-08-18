use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_coexistgate"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn analyze_fixture(name: &str) -> (i32, String) {
    let root = repo_root();
    let out = Command::new(bin())
        .args([
            "analyze",
            "--base-dir",
            root.join(format!("demo/scenarios/{name}/base"))
                .to_str()
                .unwrap(),
            "--head-dir",
            root.join(format!("demo/scenarios/{name}/head"))
                .to_str()
                .unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run cli");
    let code = out.status.code().unwrap_or(99);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (code, stdout)
}

#[test]
fn unsafe_db_fails() {
    let (code, stdout) = analyze_fixture("01-unsafe-db");
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("DB-BACKWARD-COMPAT-001"), "{stdout}");
    assert!(stdout.contains("DB-ROLLBACK-COMPAT-001"), "{stdout}");
}

#[test]
fn safe_db_passes() {
    let (code, stdout) = analyze_fixture("02-safe-db");
    assert_eq!(code, 0, "{stdout}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["gate"], "pass");
}

#[test]
fn availability_fails() {
    let (code, stdout) = analyze_fixture("03-availability");
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("DEPLOY-REPLICA-001"), "{stdout}");
}

#[test]
fn config_rollback_fails() {
    let (code, stdout) = analyze_fixture("04-config-rollback");
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("CONFIG-ROLLBACK-COMPAT-001"), "{stdout}");
}

#[test]
fn rules_and_explain() {
    let rules = Command::new(bin()).args(["rules"]).output().unwrap();
    assert!(rules.status.success());
    let text = String::from_utf8_lossy(&rules.stdout);
    assert!(text.contains("DB-BACKWARD-COMPAT-001"));
    let expl = Command::new(bin())
        .args(["explain", "DB-BACKWARD-COMPAT-001"])
        .output()
        .unwrap();
    assert!(expl.status.success());
    assert!(String::from_utf8_lossy(&expl.stdout).contains("coexist"));
}

#[test]
fn json_is_deterministic() {
    let (_, a) = analyze_fixture("01-unsafe-db");
    let (_, b) = analyze_fixture("01-unsafe-db");
    assert_eq!(a, b);
}
