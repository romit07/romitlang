use romit_ui::{build_ui, parse};
use std::path::PathBuf;

const LANDING: &str = r#"use ui

page Landing
  header
    brand "Acme"
    nav "Features" "Pricing" "Contact"
    button "Sign up"
  hero
    title "Ship faster"
    text "One line to say what it does."
    button "Get started"
  section "Features"
    card "Fast" "Compiles to plain HTML"
    card "Simple" "No setup"
  footer
    text "© Acme"
"#;

#[test]
fn landing_parses() {
    let file = parse(LANDING, "landing.rmt").expect("parse");
    assert!(file.uses_ui);
    assert_eq!(file.template_name.as_deref(), Some("default"));
}

#[test]
fn landing_builds_golden() {
    let dir = tempfile_dir("romit-ui-golden");
    build_ui(LANDING, "landing.rmt", &dir).expect("build");
    let html = std::fs::read_to_string(dir.join("index.html")).unwrap();
    let css = std::fs::read_to_string(dir.join("style.css")).unwrap();
    assert!(html.contains("Ship faster"));
    assert!(html.contains("&copy; Acme") || html.contains("© Acme"));
    assert!(!html.contains("<script"));
    assert!(css.contains(":root"));
    assert!(css.contains("--color-primary"));
    insta_snapshot(&html, &css);
}

#[test]
fn unknown_part_lists_valid() {
    let src = "use ui\n\npage X\n  widget\n";
    let err = parse(src, "bad.rmt").unwrap_err();
    let msg = err[0].to_string();
    assert!(msg.contains("U001"));
    assert!(msg.contains("page"));
    assert!(msg.contains("header"));
}

#[test]
fn unknown_theme_token_lists_valid() {
    let src = r##"use ui

page X
  theme
    color madeup "#fff"
"##;
    let dir = tempfile_dir("romit-ui-bad-token");
    let build_err = build_ui(src, "bad.rmt", &dir).unwrap_err();
    let msg = build_err[0].to_string();
    assert!(msg.contains("U006"));
    assert!(msg.contains("color-primary"));
}

fn tempfile_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn insta_snapshot(html: &str, css: &str) {
    // Deterministic golden checks (no external crate)
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("class=\"hero-title\">Ship faster</h1>"));
    assert!(html.contains("class=\"card-title\">Fast</h3>"));
    assert!(html.contains("class=\"card-body\">Compiles to plain HTML</p>"));
    assert!(css.contains("@media (max-width: 640px)"));
}
