pub struct MakefileAst {
    pub nodes: Vec<AstNode>,
}

pub enum AstNode {
    Placeholder,
}

pub fn parse(_tokens: Vec<crate::lexer::Token>) -> MakefileAst {
    todo!("implement in Task 3")
}
