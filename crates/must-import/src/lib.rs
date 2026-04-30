pub mod lexer;
pub mod parser;
pub mod translate;
pub mod writer;
pub mod report;

pub use translate::ImportResult;

pub fn import(input: &str) -> ImportResult {
    let tokens = lexer::tokenize(input);
    let ast = parser::parse(tokens);
    translate::translate(ast)
}
