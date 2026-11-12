//! Romit apps: SwiftUI-style reactive UI (`app`, `screen`, `state`, `column`, `button … do …`).
//!
//! An app compiles to one self-contained `index.html` (markup, CSS, and a small runtime).

pub mod ast;
pub mod codegen;
pub mod expr;
pub mod lex;
pub mod parse;
pub mod vocab;

use crate::diag::Diagnostic;

const RUNTIME_JS: &str = include_str!("runtime.js");
const RUNTIME_CSS: &str = include_str!("runtime.css");

/// True when a `use ui` file describes an app (has `app`, `screen`, or `state` at the top level).
pub fn is_app_source(source: &str) -> bool {
    source.lines().any(|l| {
        let first = l.split_whitespace().next().unwrap_or("");
        !l.starts_with(' ') && matches!(first, "app" | "screen" | "state")
    })
}

/// Compile app source to JavaScript (runtime not included).
pub fn compile_js(source: &str, file: &str) -> Result<String, Vec<Diagnostic>> {
    let app = parse::parse_app(source, file)?;
    let mut cx = parse::Ctx {
        file,
        source,
        diags: Vec::new(),
    };
    let js = {
        let mut g = codegen::Gen::new(&mut cx, &app);
        g.app(&app)
    };
    if !cx.diags.is_empty() {
        return Err(cx.diags);
    }
    Ok(js)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Compile app source to a complete HTML document.
pub fn build_app_html(source: &str, file: &str) -> Result<String, Vec<Diagnostic>> {
    let js = compile_js(source, file)?;
    let name = parse::parse_app(source, file)
        .map(|a| a.name)
        .unwrap_or_else(|_| "App".into());
    let script = format!(
        "(function () {{\n{RUNTIME_JS}\n/* ---- compiled from {} ---- */\n{js}\nR.start();\n}})();",
        escape_html(file)
    )
    .replace("</script", "<\\/script");
    Ok(format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\" />\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\" />\n<meta name=\"generator\" content=\"romit {}\" />\n<title>{}</title>\n<style>\n{RUNTIME_CSS}</style>\n</head>\n<body>\n<div id=\"romit-app\"></div>\n<noscript>This Romit app needs JavaScript.</noscript>\n<script>\n{script}\n</script>\n</body>\n</html>\n",
        env!("CARGO_PKG_VERSION"),
        escape_html(&name),
    ))
}
