use romit_compiler::{compile_file, format_diagnostics, interp::Interpreter};
use std::env;
use std::fs;
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        eprintln!("usage: romit check <file.rmt>");
        eprintln!("       romit test <file.rmt>");
        process::exit(2);
    };
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
    let interp = Interpreter::new(compiled.program);
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
