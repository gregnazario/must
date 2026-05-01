pub mod lexer;
pub mod parser;
pub(crate) mod report;
pub mod translate;
pub(crate) mod writer;

pub use translate::ImportResult;

pub fn import(input: &str) -> ImportResult {
    let tokens = lexer::tokenize(input);
    let ast = parser::parse(tokens);
    translate::translate(ast)
}
