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
fn script_rejects_equals_assignment() {
    let src = "x = 5\nsay x";
    let r = Parser::parse_file("eq.rmt", src);
    assert!(r.is_err());
    let msgs = romit_compiler::format_diagnostics(&r.unwrap_err());
    assert!(msgs.contains("P010"));
    assert!(msgs.contains("set x to value"));
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
