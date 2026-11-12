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
    run_program_with_input(source, "")
}

/// Run a script; each line of `input` answers the next `ask "…"`.
/// Output printed before an error is kept.
#[wasm_bindgen]
pub fn run_program_with_input(source: &str, input: &str) -> String {
    use romit_compiler::interp::{set_ask_source, set_say_sink};
    use std::cell::RefCell;
    use std::rc::Rc;

    let file = "playground.rmt";
    let printed = Rc::new(RefCell::new(String::new()));
    let sink = printed.clone();
    set_say_sink(Some(Box::new(move |line: &str| {
        let mut s = sink.borrow_mut();
        s.push_str(line);
        s.push('\n');
    })));
    let mut answers: std::collections::VecDeque<String> = input.lines().map(String::from).collect();
    let echo = printed.clone();
    set_ask_source(Some(Box::new(move |question: &str| {
        let answer = answers.pop_front();
        // show the exchange like a terminal would
        let mut s = echo.borrow_mut();
        s.push_str(question);
        s.push(' ');
        s.push_str(answer.as_deref().unwrap_or("(no input)"));
        s.push('\n');
        answer
    })));
    let result = run_source(source, file, 200_000);
    set_say_sink(None);
    set_ask_source(None);
    let stdout = printed.borrow().clone();
    let resp = match result {
        Ok(out) => ApiResponse {
            ok: true,
            stdout: stdout + &out.stdout,
            diagnostics: String::new(),
            steps: out.steps,
        },
        Err(diags) => ApiResponse {
            ok: false,
            stdout,
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
    fn ask_reads_input_lines() {
        let json = run_program_with_input("name = ask \"Name?\"\nsay \"Hi {name}\"\n", "Ada\n");
        assert!(json.contains("Hi Ada"), "{json}");
        assert!(json.contains("Name? Ada"), "{json}");
    }

    #[test]
    fn output_before_an_error_is_kept() {
        let json = run_program("say \"before\"\nfail \"stop here\"\n");
        assert!(json.contains("before"), "{json}");
        assert!(json.contains("E901"), "{json}");
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
