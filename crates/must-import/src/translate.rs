pub struct ImportResult {
    pub toml: String,
    pub report: String,
    pub translated_count: usize,
    pub skipped_count: usize,
    pub todo_count: usize,
}

pub(crate) fn translate(_ast: crate::parser::MakefileAst) -> ImportResult {
    todo!("implement in Task 4")
}
