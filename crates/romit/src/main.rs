use romit_compiler::{check::CheckedUnit, compile_file, format_diagnostics, interp::Interpreter};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

fn print_usage() {
    eprintln!("romit {}", env!("CARGO_PKG_VERSION"));
    eprintln!("usage: romit [--version] check <file.rmt>");
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

fn run_file(path: &str) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", path, e);
            return false;
        }
    };
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
