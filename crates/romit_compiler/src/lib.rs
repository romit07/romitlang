pub mod ast;
pub mod check;
pub mod diag;
pub mod interp;
pub mod keywords;
pub mod lexer;
pub mod parser;
pub mod replay;
pub mod types;

use check::{check, CheckedProgram};
use diag::Diagnostic;
use parser::Parser;
use std::collections::HashSet;

pub struct CompileResult {
    pub program: CheckedProgram,
    pub warnings: Vec<Diagnostic>,
}

pub fn compile_file(
    file: &str,
    source: &str,
    skip_coverage: bool,
) -> Result<CompileResult, Vec<Diagnostic>> {
    let spec = match Parser::parse_file(file, source) {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    let program = match check(spec, file, source) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    let mut warnings = Vec::new();
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

    Ok(CompileResult { program, warnings })
}

pub fn format_diagnostics(diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
