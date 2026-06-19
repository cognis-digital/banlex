//! Integration tests that drive the compiled `banlex` binary end to end.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_banlex"))
}

#[test]
fn scan_clear_exits_zero() {
    let out = bin()
        .args(["scan", "the quarterly report looks fine"])
        .output()
        .expect("run banlex");
    assert!(out.status.success(), "expected exit 0 for clean text");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CLEAR"));
}

#[test]
fn scan_flagged_exits_one() {
    let out = bin()
        .args(["scan", "--threshold", "0.5", "you have won, send gift card"])
        .output()
        .expect("run banlex");
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1 for flagged text"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("FLAGGED"));
}

#[test]
fn scan_json_is_valid_json() {
    let out = bin()
        .args(["scan", "--json", "buy now buy now"])
        .output()
        .expect("run banlex");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON output");
    assert!(v.get("category_scores").is_some());
    assert!(v.get("total_score").is_some());
}

#[test]
fn categories_lists_default() {
    let out = bin().arg("categories").output().expect("run banlex");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("spam"));
    assert!(stdout.contains("scam"));
    assert!(stdout.contains("harassment"));
    assert!(stdout.contains("self-harm-keywords"));
}

#[test]
fn custom_lexicon_loads() {
    let out = bin()
        .args([
            "categories",
            "--lexicon",
            "examples/lexicon.json",
            "--json",
        ])
        .output()
        .expect("run banlex");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(v["lexicon"], "example-marketing-risk");
}

#[test]
fn scan_from_file() {
    let out = bin()
        .args(["scan", "--file", "examples/texts/scam_multi.txt", "--threshold", "1.0"])
        .output()
        .expect("run banlex");
    assert_eq!(out.status.code(), Some(1), "scam sample should flag");
}

#[test]
fn missing_lexicon_errors_with_code_two() {
    let out = bin()
        .args(["scan", "--lexicon", "does_not_exist.json", "hello"])
        .output()
        .expect("run banlex");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn bad_threshold_errors() {
    let out = bin()
        .args(["scan", "--threshold", "abc", "hello"])
        .output()
        .expect("run banlex");
    assert_eq!(out.status.code(), Some(2));
}
