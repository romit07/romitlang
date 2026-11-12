use romit_compiler::{check::CheckedUnit, compile_file, format_diagnostics, interp::Interpreter};
use romit_ui::{
    build, default_out_dir, format_diagnostics as ui_format, is_ui_source, serve_preview, BuildKind,
};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

fn print_usage() {
    eprintln!("romit {}", env!("CARGO_PKG_VERSION"));
    eprintln!("usage: romit [--version] check [--no-coverage] [--json] [--stdin] <file.rmt>");
    eprintln!("       romit build <file.rmt> [--out DIR]");
    eprintln!("       romit run <file.rmt>");
    eprintln!("       romit test <file.rmt>");
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        print_usage();
        process::exit(2);
    };
    if cmd == "--version" || cmd == "-V" {
        println!("romit {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    match cmd.as_str() {
        "check" => {
            let mut skip_coverage = false;
            let mut json = false;
            let mut from_stdin = false;
            let mut path = None;
            for arg in args.by_ref() {
                match arg.as_str() {
                    "--no-coverage" => skip_coverage = true,
                    "--json" => json = true,
                    "--stdin" => from_stdin = true,
                    _ if path.is_none() => path = Some(arg),
                    other => {
                        eprintln!("romit check: unknown option {other}");
                        process::exit(2);
                    }
                }
            }
            let Some(path) = path else {
                eprintln!("romit check: missing file");
                process::exit(2);
            };
            if json {
                // For editors: diagnostics as JSON. With --stdin the text comes from stdin
                // (unsaved changes) and the path is only used for messages and `use`.
                let source = if from_stdin {
                    let mut s = String::new();
                    if io::Read::read_to_string(&mut io::stdin(), &mut s).is_err() {
                        process::exit(2);
                    }
                    s
                } else {
                    match fs::read_to_string(&path) {
                        Ok(s) => s,
                        Err(e) => {
                            println!(
                                "[{}]",
                                json_diag(
                                    &path,
                                    1,
                                    1,
                                    "E000",
                                    &format!("cannot read {path}: {e}"),
                                    None,
                                    "error"
                                )
                            );
                            process::exit(1);
                        }
                    }
                };
                let ok = run_check_json(&path, &source, skip_coverage);
                process::exit(if ok { 0 } else { 1 });
            }
            if !run_check(&path, skip_coverage) {
                process::exit(1);
            }
        }
        "build" => {
            let Some(path) = args.next() else {
                eprintln!("romit build: missing file");
                process::exit(2);
            };
            let mut out_dir = None;
            while let Some(arg) = args.next() {
                if arg == "--out" {
                    out_dir = args.next();
                } else {
                    eprintln!("romit build: unknown option {arg}");
                    process::exit(2);
                }
            }
            if !run_build(&path, out_dir.as_deref()) {
                process::exit(1);
            }
        }
        "run" => {
            let Some(path) = args.next() else {
                eprintln!("romit run: missing file");
                process::exit(2);
            };
            if !run_file(&path) {
                process::exit(1);
            }
        }
        "test" => {
            let Some(path) = args.next() else {
                eprintln!("romit test: missing file");
                process::exit(2);
            };
            if !run_test(&path) {
                process::exit(1);
            }
        }
        _ => {
            eprintln!("unknown command: {}", cmd);
            print_usage();
            process::exit(2);
        }
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_diag(
    file: &str,
    line: usize,
    column: usize,
    code: &str,
    message: &str,
    help: Option<&str>,
    severity: &str,
) -> String {
    format!(
        "{{\"file\":{},\"line\":{},\"column\":{},\"code\":{},\"message\":{},\"help\":{},\"severity\":{}}}",
        json_str(file),
        line,
        column,
        json_str(code),
        json_str(message),
        help.map(json_str).unwrap_or_else(|| "null".into()),
        json_str(severity)
    )
}

/// `romit check --json`: print every diagnostic as one JSON array (for editors).
fn run_check_json(path: &str, source: &str, skip_coverage: bool) -> bool {
    let mut items = Vec::new();
    if is_ui_source(source) {
        let result = if romit_ui::app::is_app_source(source) {
            romit_ui::app::compile_js(source, path).map(|_| ())
        } else {
            romit_ui::parse(source, path).map(|_| ())
        };
        if let Err(diags) = result {
            for d in diags {
                items.push(json_diag(
                    &d.file,
                    d.line.max(1),
                    d.column.max(1),
                    &d.code,
                    &d.message,
                    d.help.as_deref(),
                    "error",
                ));
            }
        }
    } else {
        match compile_file(path, source, skip_coverage) {
            Ok(res) => {
                for d in res.warnings {
                    items.push(json_diag(
                        &d.file,
                        d.line.max(1),
                        d.column.max(1),
                        &d.code,
                        &d.message,
                        d.help.as_deref(),
                        "warning",
                    ));
                }
            }
            Err(diags) => {
                for d in diags {
                    let sev = if d.severity == romit_compiler::diag::Severity::Warning {
                        "warning"
                    } else {
                        "error"
                    };
                    items.push(json_diag(
                        &d.file,
                        d.line.max(1),
                        d.column.max(1),
                        &d.code,
                        &d.message,
                        d.help.as_deref(),
                        sev,
                    ));
                }
            }
        }
    }
    let ok = !items.iter().any(|i| i.contains("\"severity\":\"error\""));
    println!("[{}]", items.join(","));
    ok
}

fn run_check(path: &str, skip_coverage: bool) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", path, e);
            return false;
        }
    };
    if is_ui_source(&source) {
        if romit_ui::app::is_app_source(&source) {
            return match romit_ui::app::compile_js(&source, path) {
                Ok(_) => {
                    println!("{}: ok (app)", path);
                    true
                }
                Err(diags) => {
                    eprint!("{}", ui_format(&diags));
                    false
                }
            };
        }
        println!("{}: ui page (use romit build)", path);
        return true;
    }
    match compile_file(path, &source, skip_coverage) {
        Ok(res) => {
            for w in &res.warnings {
                eprint!("{}", w);
            }
            println!("{}: ok", path);
            true
        }
        Err(diags) => {
            eprint!("{}", format_diagnostics(&diags));
            false
        }
    }
}

fn run_build(path: &str, out: Option<&str>) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", path, e);
            return false;
        }
    };
    if !is_ui_source(&source) {
        eprintln!("error: {} is not a Romit UI file (missing `use ui`)", path);
        return false;
    }
    let out_path = match out {
        Some(d) => std::path::PathBuf::from(d),
        None => default_out_dir(Path::new(path)),
    };
    match build(&source, path, &out_path) {
        Ok(BuildKind::Page) => {
            println!(
                "{}: wrote {}/index.html and {}/style.css",
                path,
                out_path.display(),
                out_path.display()
            );
            true
        }
        Ok(BuildKind::App) => {
            println!(
                "{}: wrote {}/index.html (open it in a browser, or `romit run {}` to preview)",
                path,
                out_path.display(),
                path
            );
            true
        }
        Err(diags) => {
            eprint!("{}", ui_format(&diags));
            false
        }
    }
}

fn run_file(path: &str) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", path, e);
            return false;
        }
    };
    if is_ui_source(&source) {
        let port = romit_ui::preview::pick_port();
        if let Err(e) = serve_preview(Path::new(path), port) {
            eprintln!("error: preview server: {e}");
            return false;
        }
        return true;
    }
    // Print as the script runs, and answer `ask` from the keyboard (or piped input).
    romit_compiler::interp::set_say_sink(Some(Box::new(|line: &str| {
        let mut out = io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    })));
    romit_compiler::interp::set_ask_source(Some(Box::new(|question: &str| {
        if !question.is_empty() {
            let mut out = io::stdout().lock();
            let _ = write!(out, "{question} ");
            let _ = out.flush();
        }
        let mut answer = String::new();
        match io::stdin().read_line(&mut answer) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(answer),
        }
    })));
    match romit_compiler::run_source(&source, path, 1_000_000) {
        Ok(outcome) => {
            print!("{}", outcome.stdout);
            let _ = io::stdout().flush();
            true
        }
        Err(diags) => {
            eprint!("{}", format_diagnostics(&diags));
            false
        }
    }
}

fn run_test(path: &str) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", path, e);
            return false;
        }
    };
    if is_ui_source(&source) {
        eprintln!("{}: ui file (no tests yet)", path);
        return true;
    }
    let compiled = match compile_file(path, &source, true) {
        Ok(c) => c,
        Err(diags) => {
            eprint!("{}", format_diagnostics(&diags));
            return false;
        }
    };
    let CheckedUnit::Spec(program) = compiled.unit else {
        println!("{}: script (no tests)", path);
        return true;
    };
    let interp = Interpreter::new(program);
    match interp.run_tests(path, &source) {
        Ok(passed) => {
            println!("{}: {} test(s) passed", path, passed.len());
            true
        }
        Err(diags) => {
            eprint!("{}", format_diagnostics(&diags));
            false
        }
    }
}
