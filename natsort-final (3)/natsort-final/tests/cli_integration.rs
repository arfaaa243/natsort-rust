//! CLI integration tests: spawn the actual built `natsort_port` binary and
//! check its stdin/stdout/stderr/exit-code behavior end-to-end, rather
//! than calling `natsort_core` functions directly (that's what the crate's
//! own `#[cfg(test)]` modules and the other files under `tests/` are for).
//!
//! Uses only `std::process::Command` — no `assert_cmd` or similar, keeping
//! this offline-verifiable without any new dev-dependency (consistent
//! with this crate's zero-dependency stance; see DECISIONS.md #7).

use std::io::Write;
use std::process::{Command, Stdio};

/// Run the built binary with `args`, feeding `stdin_data` on stdin.
/// Returns `(stdout, stderr, exit_code)`.
fn run(args: &[&str], stdin_data: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_natsort_port"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn natsort_port");
    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(stdin_data.as_bytes())
        .expect("failed to write stdin");
    let output = child.wait_with_output().expect("failed to wait on child");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

#[test]
fn sort_default_orders_numerically() {
    let (stdout, _stderr, code) = run(&["sort"], "file10\nfile2\nfile1\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "file1\nfile2\nfile10\n");
}

#[test]
fn sort_reverse_flag_reverses_final_order() {
    let (stdout, _stderr, code) = run(&["sort", "--reverse"], "a\nb\nc\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "c\nb\na\n");
}

#[test]
fn sort_real_flag_parses_floats() {
    let (stdout, _stderr, code) = run(&["sort", "--real"], "1.10\n1.2\n1.5\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "1.10\n1.2\n1.5\n");
}

#[test]
fn sort_signed_flag_orders_negative_numbers_correctly() {
    let (stdout, _stderr, code) = run(&["sort", "--signed"], "item-3\nitem-10\nitem2\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "item-10\nitem-3\nitem2\n");
}

#[test]
fn sort_ignorecase_flag_folds_case() {
    let (stdout, _stderr, code) = run(&["sort", "--ignorecase"], "Banana\napple\nCherry\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "apple\nBanana\nCherry\n");
}

#[test]
fn compare_prints_less_equal_greater() {
    let (stdout, _stderr, code) = run(&["compare"], "file2\nfile10\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "<\n");

    let (stdout, _stderr, code) = run(&["compare"], "same\nsame\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "=\n");

    let (stdout, _stderr, code) = run(&["compare"], "file10\nfile2\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, ">\n");
}

#[test]
fn compare_wrong_line_count_is_a_usage_error() {
    let (_stdout, stderr, code) = run(&["compare"], "onlyoneline\n");
    assert_eq!(code, 2);
    assert!(stderr.contains("exactly two lines"));
}

#[test]
fn compare_too_many_lines_is_a_usage_error() {
    let (_stdout, stderr, code) = run(&["compare"], "a\nb\nc\n");
    assert_eq!(code, 2);
    assert!(stderr.contains("exactly two lines"));
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    let (_stdout, stderr, code) = run(&["bogus"], "");
    assert_eq!(code, 2);
    assert!(stderr.contains("unknown subcommand"));
}

#[test]
fn unrecognized_flag_is_a_usage_error() {
    let (_stdout, stderr, code) = run(&["sort", "--nope"], "a\nb\n");
    assert_eq!(code, 2);
    assert!(stderr.contains("unrecognized flag"));
}

#[test]
fn no_args_is_a_usage_error() {
    let (_stdout, _stderr, code) = run(&[], "");
    assert_eq!(code, 2);
}

#[test]
fn help_flag_prints_usage_and_exits_success() {
    let (_stdout, stderr, code) = run(&["--help"], "");
    assert_eq!(code, 0);
    assert!(stderr.contains("usage"));

    let (_stdout, stderr, code) = run(&["-h"], "");
    assert_eq!(code, 0);
    assert!(stderr.contains("usage"));
}

#[test]
fn version_flag_prints_version_and_exits_success() {
    let (stdout, _stderr, code) = run(&["--version"], "");
    assert_eq!(code, 0);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));

    let (stdout, _stderr, code) = run(&["-V"], "");
    assert_eq!(code, 0);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn sort_reads_from_a_file_argument_instead_of_stdin() {
    let path = std::env::temp_dir().join(format!(
        "natsort_cli_test_file_{}_{}.txt",
        std::process::id(),
        "sort_reads_from_file"
    ));
    std::fs::write(&path, "b\na\nc\n").expect("failed to write temp file");
    let (stdout, _stderr, code) = run(&["sort", path.to_str().unwrap()], "ignored-stdin-data\n");
    std::fs::remove_file(&path).ok();
    assert_eq!(code, 0);
    assert_eq!(stdout, "a\nb\nc\n");
}

#[test]
fn sort_missing_file_argument_is_an_io_error() {
    let (_stdout, stderr, code) = run(
        &["sort", "/nonexistent/path/natsort_port_does_not_exist.txt"],
        "",
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("failed to read"));
}

#[test]
fn sort_extra_positional_argument_is_a_usage_error() {
    let (_stdout, stderr, code) = run(&["sort", "one.txt", "two.txt"], "");
    assert_eq!(code, 2);
    assert!(stderr.contains("unexpected extra argument"));
}

#[test]
fn empty_stdin_sorts_to_empty_output() {
    let (stdout, _stderr, code) = run(&["sort"], "");
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn sort_output_is_stable_for_duplicate_lines() {
    let (stdout, _stderr, code) = run(&["sort"], "b\na\nb\na\nb\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "a\na\nb\nb\nb\n");
}
