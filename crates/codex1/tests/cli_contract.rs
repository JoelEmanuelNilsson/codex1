use assert_cmd::Command;
use serde_json::Value;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

#[test]
fn clap_argument_errors_emit_json_envelope_and_exit_one() {
    let output = cmd()
        .args(["task", "finish", "T1", "--mission", "demo"])
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stderr.is_empty(),
        "stderr should stay empty, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("expected JSON stdout, got {stdout:?}: {err}"));
    assert_eq!(json["ok"], false);
    assert_eq!(json["code"], "PARSE_ERROR");
    assert!(
        json["message"]
            .as_str()
            .unwrap_or_default()
            .contains("--proof"),
        "{json}"
    );
}
