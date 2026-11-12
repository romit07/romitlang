use romit_compiler::check::CheckedUnit;
use romit_compiler::interp::Interpreter;
use romit_compiler::{compile_file, format_diagnostics, run_source};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
    stdout: String,
    diagnostics: String,
    steps: u64,
}

#[wasm_bindgen]
pub fn romit_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn check_program(source: &str) -> String {
    let file = "playground.rmt";
    let res = compile_file(file, source, true);
    let resp = match res {
        Ok(_) => ApiResponse {
            ok: true,
            stdout: String::new(),
            diagnostics: String::new(),
            steps: 0,
        },
        Err(diags) => ApiResponse {
            ok: false,
            stdout: String::new(),
            diagnostics: format_diagnostics(&diags),
            steps: 0,
        },
    };
    serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into())
}

#[wasm_bindgen]
pub fn test_program(source: &str) -> String {
    let file = "playground.rmt";
    let resp = match compile_file(file, source, true) {
        Ok(c) => match c.unit {
            CheckedUnit::Spec(program) => {
                let interp = Interpreter::new(program);
                match interp.run_tests(file, source) {
                    Ok(passed) => ApiResponse {
                        ok: true,
                        stdout: format!("{} test(s) passed\n", passed.len()),
                        diagnostics: String::new(),
                        steps: passed.len() as u64,
                    },
                    Err(diags) => ApiResponse {
                        ok: false,
                        stdout: String::new(),
                        diagnostics: format_diagnostics(&diags),
                        steps: 0,
                    },
                }
            }
            CheckedUnit::Script(_) => ApiResponse {
                ok: false,
                stdout: String::new(),
                diagnostics: "use Run on scripts; this program is a script with no tests".into(),
                steps: 0,
            },
        },
        Err(diags) => ApiResponse {
            ok: false,
            stdout: String::new(),
            diagnostics: format_diagnostics(&diags),
            steps: 0,
        },
    };
    serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into())
}

#[wasm_bindgen]
pub fn run_program(source: &str) -> String {
    let file = "playground.rmt";
    let resp = match run_source(source, file, 50_000) {
        Ok(out) => ApiResponse {
            ok: true,
            stdout: out.stdout,
            diagnostics: String::new(),
            steps: out.steps,
        },
        Err(diags) => ApiResponse {
            ok: false,
            stdout: String::new(),
            diagnostics: format_diagnostics(&diags),
            steps: 0,
        },
    };
    serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into())
}

#[derive(Serialize)]
struct UiResponse {
    ok: bool,
    html: String,
    diagnostics: String,
}

/// True when the source is a Romit UI file (`use ui`) that should be previewed, not run.
#[wasm_bindgen]
pub fn is_ui_program(source: &str) -> bool {
    romit_ui::is_ui_source(source)
}

/// Compile a `use ui` file (app or page) to one HTML document for a live preview.
#[wasm_bindgen]
pub fn build_ui_program(source: &str) -> String {
    let resp = match romit_ui::preview_html(source, "playground.rmt") {
        Ok(html) => UiResponse {
            ok: true,
            html,
            diagnostics: String::new(),
        },
        Err(diags) => UiResponse {
            ok: false,
            html: String::new(),
            diagnostics: romit_ui::format_diagnostics(&diags),
        },
    };
    serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world_script_json() {
        let src = "say \"Hello, world!\"\n";
        let json = run_program(src);
        assert!(json.contains("Hello, world!"));
        assert!(json.contains("\"ok\":true"));
    }

    #[test]
    fn ui_app_preview_json() {
        let src = "use ui\nstate\n  n is 0\nscreen Home\n  button \"+\" do n += 1\n";
        assert!(is_ui_program(src));
        let json = build_ui_program(src);
        assert!(json.contains("\"ok\":true"), "{json}");
        assert!(json.contains("R.start()"));
    }
}
