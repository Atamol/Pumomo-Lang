pub mod ast;
pub mod error;
pub mod interp;
pub mod lexer;
pub mod parser;
pub mod pumomo;
pub mod value;

use error::PResult;

pub fn run_source(src: &str) -> PResult<i32> {
    let toks = lexer::lex(src)?;
    let prog = parser::parse(toks)?;
    interp::run(&prog)
}
