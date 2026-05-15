use std::path::Path;

use llm_wiki_schema::SkillDoc;

pub const BINARY_MARKER: &str = "{llm_wiki_binary}";

pub fn apply_binary_context(mut doc: SkillDoc, binary: &str) -> SkillDoc {
    doc.body.purpose = doc.body.purpose.replace(BINARY_MARKER, binary);
    doc.body.behavior = doc.body.behavior.replace(BINARY_MARKER, binary);
    doc.body.invocation = doc.body.invocation.replace(BINARY_MARKER, binary);
    doc.body.notes = doc
        .body
        .notes
        .map(|notes| notes.replace(BINARY_MARKER, binary));
    doc
}

pub fn managed_binary_invocation(path: &Path) -> String {
    if cfg!(windows) {
        format!("& {}", powershell_single_quote(&path.display().to_string()))
    } else {
        shell_single_quote(&path.display().to_string())
    }
}

fn powershell_single_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "''"))
}

fn shell_single_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::{powershell_single_quote, shell_single_quote};

    #[test]
    fn quotes_shell_paths() {
        assert_eq!(shell_single_quote("/tmp/llm wiki"), "'/tmp/llm wiki'");
        assert_eq!(shell_single_quote("/tmp/llm'wiki"), "'/tmp/llm'\\''wiki'");
    }

    #[test]
    fn quotes_powershell_paths() {
        assert_eq!(
            powershell_single_quote(r"C:\Users\Test User\AppData\Local\llm_wiki\bin\llm-wiki.exe"),
            r"'C:\Users\Test User\AppData\Local\llm_wiki\bin\llm-wiki.exe'"
        );
        assert_eq!(
            powershell_single_quote(r"C:\Users\O'Hara\bin\llm-wiki.exe"),
            r"'C:\Users\O''Hara\bin\llm-wiki.exe'"
        );
    }
}
