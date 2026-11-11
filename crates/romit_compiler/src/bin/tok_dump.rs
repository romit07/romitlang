use romit_compiler::lexer::Lexer;
use std::env;

fn main() {
    let path = env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&path).unwrap();
    let toks = Lexer::new(src).tokenize(&path).unwrap();
    for (i, t) in toks.iter().enumerate() {
        println!("{} {:?} @{}:{}", i, t.kind, t.line, t.column);
    }
}
