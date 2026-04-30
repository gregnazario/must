use crate::translate::{MustfileOutput, TodoKind};

pub(crate) fn write_report(output: &MustfileOutput, translated: usize, todo: usize, skipped: usize) -> String {
    let mut out = String::new();

    out.push_str("# Mustfile Import Report\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- Translated: {translated} items\n"));
    out.push_str(&format!("- TODO (manual review needed): {todo} items\n"));
    out.push_str(&format!("- Skipped (unrecognized): {skipped} items\n"));

    if !output.env.is_empty() || !output.recipes.is_empty() {
        out.push_str("\n## Translated\n\n");
        for (k, _) in &output.env {
            out.push_str(&format!("- Variable `{k}`\n"));
        }
        for r in &output.recipes {
            if r.phony {
                out.push_str(&format!("- Rule `{}` (phony)\n", r.name));
            } else {
                out.push_str(&format!("- Rule `{}`\n", r.name));
            }
        }
    }

    if !output.todos.is_empty() {
        out.push_str("\n## TODO — Needs Manual Review\n\n");
        for item in &output.todos {
            match item.kind {
                TodoKind::PatternRule => {
                    out.push_str(&format!("- Pattern rule: `{}` — pattern rules are not supported; convert manually\n", item.description));
                }
                TodoKind::Include => {
                    out.push_str(&format!("- Include: `{}` — include directives are not supported; inline the file\n", item.description));
                }
            }
        }
    }

    if !output.skipped.is_empty() {
        out.push_str("\n## Skipped\n\n");
        for s in &output.skipped {
            out.push_str(&format!("- `{s}`\n"));
        }
    }

    out
}
