use crate::doc::ParseError;
use crate::frontmatter::{InvocationStyle, Runtime, SkillFrontmatter};

pub fn validate(frontmatter: &SkillFrontmatter) -> Result<(), ParseError> {
    require_non_empty("name", &frontmatter.name)?;
    require_non_empty("description", &frontmatter.description)?;
    if frontmatter.runtimes.is_empty() {
        return Err(ParseError::Validation(
            "runtimes must not be empty".to_string(),
        ));
    }
    for operation in &frontmatter.operations {
        require_non_empty("operations[]", operation)?;
    }
    for argument in &frontmatter.arguments {
        require_non_empty("arguments[].name", &argument.name)?;
        require_non_empty("arguments[].description", &argument.description)?;
    }
    for operation in &frontmatter.dispatcher_for {
        require_non_empty("dispatcher_for[]", operation)?;
    }
    let is_codex = frontmatter.runtimes.contains(&Runtime::Codex);
    if !frontmatter.dispatcher_for.is_empty() && !is_codex {
        return Err(ParseError::Validation(
            "dispatcher_for requires the codex runtime".to_string(),
        ));
    }
    if matches!(
        frontmatter.invocation_style,
        Some(InvocationStyle::Dispatch)
    ) && frontmatter.dispatcher_for.is_empty()
    {
        return Err(ParseError::Validation(
            "dispatch invocation_style requires dispatcher_for".to_string(),
        ));
    }
    Ok(())
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ParseError> {
    if value.trim().is_empty() {
        return Err(ParseError::Validation(format!("{field} must not be empty")));
    }
    Ok(())
}
