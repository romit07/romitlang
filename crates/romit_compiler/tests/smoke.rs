use romit_compiler::{compile_file, lexer::Lexer, parser::Parser};

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
    let spec = Parser::parse_file("traffic_light.rmt", src).unwrap();
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
    let spec = Parser::parse_file("fibonacci.rmt", src).unwrap();
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
