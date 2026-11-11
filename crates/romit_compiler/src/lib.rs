pub mod ast;
pub mod check;
pub mod diag;
pub mod interp;
pub mod keywords;
pub mod lexer;
pub mod parser;
pub mod replay;
pub mod suggest;
pub mod types;

use check::{check_program, CheckedUnit};
use diag::Diagnostic;
use parser::Parser;
use std::collections::HashSet;

pub struct CompileResult {
    pub unit: CheckedUnit,
    pub warnings: Vec<Diagnostic>,
}

pub fn compile_file(
    file: &str,
    source: &str,
    skip_coverage: bool,
) -> Result<CompileResult, Vec<Diagnostic>> {
    let program = match Parser::parse_file(file, source) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    let unit = match check_program(program, file, source) {
        Ok(u) => u,
        Err(e) => return Err(e),
    };

    if unit.is_script() {
        return Ok(CompileResult {
            unit,
            warnings: Vec::new(),
        });
    }

    let program = unit.as_spec().expect("spec").clone();

    let warnings = Vec::new();
    let mut coverage_errs = Vec::new();

    if !skip_coverage {
        let interp = interp::Interpreter::new(program.clone());
        let mut fired: HashSet<(String, String)> = HashSet::new();
        for test in &program.tests {
            let mut rt = interp.init_from_spec();
            for g in &test.givens {
                interp.apply_given(&mut rt, g);
            }
            for step in &test.steps {
                if let crate::ast::TestStep::Send(send) = step {
                    let mut args = std::collections::HashMap::new();
                    for (k, e) in &send.args {
                        args.insert(k.clone(), interp.eval_expr(&rt, e, &interp::Env::default()));
                    }
                    if let Ok(r) = interp.run_event(&mut rt, &send.event, &args) {
                        for fr in r.fired_rules {
                            fired.insert(fr);
                        }
                    }
                }
            }
        }
        for h in &program.handlers {
            for r in &h.rules {
                if !fired.contains(&(h.event.clone(), r.name.clone())) {
                    coverage_errs.push(
                        Diagnostic::error(
                            "E402",
                            format!("no test makes rule `{}` run", r.name),
                            file,
                            r.span.line,
                            r.span.column,
                            diag::snippet_at(source, r.span.line, r.span.column, &r.name),
                        )
                        .with_help(format!(
                            "add a test that sends {}() and reaches this rule",
                            h.event
                        )),
                    );
                }
            }
        }
    }

    if !coverage_errs.is_empty() {
        return Err(coverage_errs);
    }

    Ok(CompileResult {
        unit: CheckedUnit::Spec(program),
        warnings,
    })
}

pub fn format_diagnostics(diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn check_source(
    file: &str,
    source: &str,
    skip_coverage: bool,
) -> Result<CompileResult, Vec<Diagnostic>> {
    compile_file(file, source, skip_coverage)
}

pub fn run_source(
    source: &str,
    file: &str,
    max_steps: u64,
) -> Result<interp::RunOutcome, Vec<Diagnostic>> {
    let compiled = compile_file(file, source, true)?;
    match &compiled.unit {
        CheckedUnit::Script(script) => {
            let i = interp::ScriptInterpreter::new(script.clone());
            i.run_with_limits(file, source, max_steps)
        }
        CheckedUnit::Spec(_) => Err(vec![Diagnostic::error(
            "E007",
            "only scripts can run without a spec; start with `say` or `print` lines or use `romit test`",
            file,
            1,
            1,
            diag::snippet_at(source, 1, 1, "file"),
        )]),
    }
}
