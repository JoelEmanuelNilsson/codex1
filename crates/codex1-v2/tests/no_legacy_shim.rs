//! Round 3 Fix 2 regression guard: the V2 binary must not pretend to handle
//! V1's `internal stop-hook` subcommand. If anyone re-adds a compatibility
//! shim, this test breaks.

use assert_cmd::Command;

#[test]
fn internal_stop_hook_is_rejected_as_unknown_subcommand() {
    let output = Command::cargo_bin("codex1")
        .expect("binary built")
        .args(["internal", "stop-hook"])
        .output()
        .expect("run codex1");

    assert!(
        !output.status.success(),
        "V2 must refuse `internal stop-hook`; exit status: {:?}",
        output.status
    );

    // clap emits the complaint on stderr by default.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("invalid value"),
        "expected clap unknown-subcommand error; got stderr: {stderr}"
    );

    // Exit code must be clap's usage-error code (2), not 0. A fail-open shim
    // that exited 0 would pass earlier checks but fail here.
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit 2; got {:?} (possible reintroduction of fail-open compat shim)",
        output.status.code()
    );
}

#[test]
fn no_internal_subcommand_group_exists() {
    // Even without `stop-hook`, `internal` itself must not be a recognized
    // subcommand group — there is no `internal` surface in V2.
    let output = Command::cargo_bin("codex1")
        .expect("binary built")
        .args(["internal"])
        .output()
        .expect("run codex1");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("invalid value"),
        "expected clap unknown-subcommand error; got stderr: {stderr}"
    );
}
