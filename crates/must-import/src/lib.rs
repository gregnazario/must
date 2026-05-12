pub mod justfile;
pub mod lexer;
pub mod parser;
pub(crate) mod report;
pub mod taskfile;
pub mod translate;
pub(crate) mod writer;

pub use translate::ImportResult;

pub fn import(input: &str) -> ImportResult {
    let tokens = lexer::tokenize(input);
    let ast = parser::parse(tokens);
    translate::translate(ast)
}

pub fn import_justfile(input: &str) -> ImportResult {
    let output = justfile::parse_justfile(input);
    finish_import(output)
}

pub fn import_taskfile(input: &str) -> ImportResult {
    let output = taskfile::parse_taskfile(input);
    finish_import(output)
}

fn finish_import(output: translate::MustfileOutput) -> ImportResult {
    let translated_count = output.env.len() + output.recipes.len();
    let todo_count = output.todos.len();
    let skipped_count = output.skipped.len();
    let toml = writer::write_toml(&output);
    let report = report::write_report(&output, translated_count, todo_count, skipped_count);

    ImportResult {
        toml,
        report,
        translated_count,
        todo_count,
        skipped_count,
    }
}
