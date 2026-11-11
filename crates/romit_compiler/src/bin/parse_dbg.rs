use romit_compiler::lexer::Lexer;
use romit_compiler::parser::Parser;
use std::env;
use std::time::Instant;

fn main() {
    let path = env::args().nth(1).expect("path");
    let src = std::fs::read_to_string(&path).unwrap();
    let t0 = Instant::now();
    let mut lex = Lexer::new(src.clone());
    let tokens = lex.tokenize(&path).unwrap();
    println!("tokens {} in {:?}", tokens.len(), t0.elapsed());
    let t1 = Instant::now();
    let r = Parser::parse_file(&path, &src);
    println!("parse {:?} {:?}", t1.elapsed(), r.is_ok());
    if let Err(e) = r {
        for d in e {
            println!("{}", d);
        }
    }
}
