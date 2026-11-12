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
    eprintln!("usage: romit [--version] check <file.rmt>");
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
            let Some(mut path) = args.next() else {
                eprintln!("romit check: missing file");
                process::exit(2);
            };
            if path == "--no-coverage" {
                skip_coverage = true;
                path = match args.next() {
                    Some(p) => p,
                    None => {
                        eprintln!("romit check: missing file");
                        process::exit(2);
                    }
                };
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
    match romit_compiler::run_source(&source, path, 100_000) {
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
