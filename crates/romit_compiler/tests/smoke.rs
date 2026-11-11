use romit_compiler::{ast::Program, compile_file, lexer::Lexer, parser::Parser};

#[test]
fn tokenize_hello() {
    let src = include_str!("../../../examples/beginner/hello_world.rmt");
    let mut lex = Lexer::new(src.to_string());
    let tokens = lex.tokenize("hello.rmt").unwrap();
    assert!(tokens.len() < 500, "token count {}", tokens.len());
}

#[test]
fn parses_hello() {
    let src = include_str!("../../../examples/beginner/hello_world.rmt");
    let r = Parser::parse_file("hello.rmt", src);
    assert!(r.is_ok(), "{:?}", r.err());
}

#[test]
fn parse_calculator() {
    let src = include_str!("../../../examples/beginner/calculator.rmt");
    let r = Parser::parse_file("calculator.rmt", src);
    assert!(r.is_ok(), "{:?}", r.err());
}

#[test]
fn traffic_test_count() {
    let src = include_str!("../../../examples/beginner/traffic_light.rmt");
    let program = Parser::parse_file("traffic_light.rmt", src).unwrap();
    let Program::Spec(spec) = program else {
        panic!("expected spec");
    };
    let n = spec
        .decls
        .iter()
        .filter(|d| matches!(d, romit_compiler::ast::Decl::Test(_)))
        .count();
    let steps = spec
        .decls
        .iter()
        .find_map(|d| match d {
            romit_compiler::ast::Decl::Test(t) => Some(t.steps.len()),
            _ => None,
        })
        .unwrap_or(0);
    assert_eq!(n, 2, "single test had {} steps", steps);
}

#[test]
fn fibonacci_has_three_tests() {
    let src = include_str!("../../../examples/beginner/fibonacci.rmt");
    let program = Parser::parse_file("fibonacci.rmt", src).unwrap();
    let Program::Spec(spec) = program else {
        panic!("expected spec");
    };
    let n = spec
        .decls
        .iter()
        .filter(|d| matches!(d, romit_compiler::ast::Decl::Test(_)))
        .count();
    assert_eq!(n, 3, "expected 3 test decls, decls: {:?}", spec.decls.len());
}

#[test]
fn rejects_braces_with_p013() {
    let src = include_str!("../../../examples/negative/braces.rmt");
    let mut lex = Lexer::new(src.to_string());
    assert!(lex.tokenize("braces.rmt").is_err());
}

#[test]
fn compile_hello_skips_coverage() {
    let src = include_str!("../../../examples/beginner/hello_world.rmt");
    let r = compile_file("hello_world.rmt", src, true);
    assert!(r.is_ok(), "{:?}", r.err());
}

#[test]
fn parse_script_hello() {
    let src = include_str!("../../../examples/beginner/hello.rmt");
    let r = Parser::parse_file("hello.rmt", src);
    assert!(r.is_ok(), "{:?}", r.err());
    assert!(matches!(r.unwrap(), Program::Script(_)));
}

#[test]
fn run_script_hello() {
    let src = include_str!("../../../examples/beginner/hello.rmt");
    let out = romit_compiler::run_source(src, "hello.rmt", 1000).unwrap();
    assert!(out.stdout.contains("Hello, world!"));
}

#[test]
fn script_spec_mix_error() {
    let src = "say \"hi\"\non Run\n  rule x\n    when true\n    emit Greet";
    let r = Parser::parse_file("mix.rmt", src);
    assert!(r.is_err());
}

#[test]
fn run_script_variables() {
    let src = include_str!("../../../examples/beginner/variables.rmt");
    let out = romit_compiler::run_source(src, "variables.rmt", 1000).unwrap();
    assert_eq!(out.stdout.trim(), "Total: 14\n6");
}

#[test]
fn script_undefined_variable() {
    let src = include_str!("../../../examples/negative/undefined_variable.rmt");
    let Err(diags) = romit_compiler::compile_file("undefined_variable.rmt", src, true) else {
        panic!("expected compile error");
    };
    let msgs = romit_compiler::format_diagnostics(&diags);
    assert!(msgs.contains("E108"));
    assert!(msgs.contains("mystery"));
}

#[test]
fn run_script_assign_equals() {
    let src = include_str!("../../../examples/beginner/assign_equals.rmt");
    let out = romit_compiler::run_source(src, "assign_equals.rmt", 1000).unwrap();
    assert_eq!(out.stdout.trim(), "Hi Ada\nTotal: 14\n6");
}

#[test]
fn lexer_tokenizes_tight_equals() {
    let src = "x=5\n";
    let mut lex = Lexer::new(src.to_string());
    let tokens = lex.tokenize("tight.rmt").unwrap();
    let kinds: Vec<_> = tokens
        .iter()
        .filter(|t| {
            !matches!(
                t.kind,
                romit_compiler::lexer::TokenKind::Newline | romit_compiler::lexer::TokenKind::Eof
            )
        })
        .map(|t| format!("{:?}", t.kind))
        .collect();
    assert_eq!(kinds, ["Ident(\"x\")", "Assign", "Int(5)"]);
}

#[test]
fn script_rejects_equals_in_expression() {
    let src = "say x = 5\n";
    let r = Parser::parse_file("eq_expr.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("P010") || msgs.contains("P020") || msgs.contains("="));
}

#[test]
fn script_condition_rejects_lone_equals() {
    let src = r#"spec S version 1
state
  x int is 0
event E
on E
  rule r
    when x = 5
    set x to 5
"#;
    let r = Parser::parse_file("eq_when.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("P010"));
}

#[test]
fn print_alias_script() {
    let src = include_str!("../../../examples/beginner/print_alias.rmt");
    let out = romit_compiler::run_source(src, "print_alias.rmt", 1000).unwrap();
    assert_eq!(out.stdout.trim(), "Hi\nHello, Romit\nRomit");
}

#[test]
fn say_and_print_run_same() {
    let say_out = romit_compiler::run_source("say \"Hi\"\n", "say.rmt", 100).unwrap();
    let print_out = romit_compiler::run_source("print \"Hi\"\n", "print.rmt", 100).unwrap();
    assert_eq!(say_out.stdout, print_out.stdout);
    assert_eq!(say_out.stdout.trim(), "Hi");
}

#[test]
fn script_set_spec_mix_still_e005() {
    let src = "set x to 1\nenum Color red green";
    let r = Parser::parse_file("mix2.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("E005"));
}

#[test]
fn run_sum_fn_comma_args() {
    let src = include_str!("../../../examples/beginner/sum_fn.rmt");
    let out = romit_compiler::run_source(src, "sum_fn.rmt", 1000).unwrap();
    assert_eq!(out.stdout.trim(), "7", "stdout={}", out.stdout);
}

#[test]
fn function_comma_and_space_params_parse_same() {
    let colon = "sum: a, b\n  give a plus b\n";
    let alias = "fn sum a b\n  give a plus b\n";
    let long = "function sum a, b\n  give a plus b\n";
    assert!(Parser::parse_file("c.rmt", colon).is_ok());
    assert!(Parser::parse_file("s.rmt", alias).is_ok());
    assert!(Parser::parse_file("f.rmt", long).is_ok());
}

#[test]
fn fn_no_param_def_runs() {
    let src = "fn hello\n  give \"ok\"\n\nsay hello\n";
    let out = romit_compiler::run_source(src, "hello.rmt", 100).unwrap();
    assert_eq!(out.stdout.trim(), "ok");
}

#[test]
fn fn_aliases_example_runs() {
    let src = include_str!("../../../examples/beginner/fn_aliases.rmt");
    let out = romit_compiler::run_source(src, "fn_aliases.rmt", 1000).unwrap();
    assert_eq!(out.stdout.trim(), "612");
}

#[test]
fn operators_symbols_and_words() {
    let src = r#"set x to 2+3*4
set y to 10 remainder 3
set ok to x is 14 and y==1
say ok
"#;
    let out = romit_compiler::run_source(src, "ops.rmt", 100).unwrap();
    assert_eq!(out.stdout.trim(), "true");
}

#[test]
fn say_comma_parts_unchanged_total_line() {
    let src = "set total to 14\nsay \"Total: \", total\n";
    let out = romit_compiler::run_source(src, "total.rmt", 100).unwrap();
    assert_eq!(out.stdout.trim(), "Total: 14");
}

#[test]
fn say_three_comma_parts_nested_calls() {
    let src = r#"one: n
  give n
sum: a, b
  give a plus b
say one 1, sum 2, 3
"#;
    let out = romit_compiler::run_source(src, "three.rmt", 1000).unwrap();
    assert_eq!(out.stdout.trim(), "15");
}

#[test]
fn negative_trailing_comma_p014() {
    let src = include_str!("../../../examples/negative/trailing_comma.rmt");
    let r = Parser::parse_file("trailing_comma.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("P014"));
}

#[test]
fn negative_doubled_comma_p015() {
    let src = include_str!("../../../examples/negative/doubled_comma.rmt");
    let r = Parser::parse_file("doubled_comma.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("P015"));
}

#[test]
fn negative_fn_missing_name_e119() {
    let src = include_str!("../../../examples/negative/fn_missing_name.rmt");
    let r = Parser::parse_file("fn_missing_name.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("E119"));
}

#[test]
fn negative_fn_missing_body_e118() {
    let src = include_str!("../../../examples/negative/fn_missing_body.rmt");
    let r = Parser::parse_file("fn_missing_body.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("E118"));
}

#[test]
fn negative_call_with_colon_p027() {
    let src = include_str!("../../../examples/negative/call_with_colon.rmt");
    let r = Parser::parse_file("call_with_colon.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("P027"));
}

#[test]
fn run_factorial_script() {
    let src = include_str!("../../../examples/beginner/factorial.rmt");
    let out = romit_compiler::run_source(src, "factorial.rmt", 50_000).unwrap();
    assert_eq!(out.stdout.trim(), "120");
}
