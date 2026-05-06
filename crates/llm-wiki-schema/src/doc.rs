use crate::body::SkillBody;
use crate::frontmatter::SkillFrontmatter;
use crate::validation::validate;
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillDoc {
    pub frontmatter: SkillFrontmatter,
    pub body: SkillBody,
}

pub fn parse(input: &str) -> Result<SkillDoc, ParseError> {
    let (frontmatter, body) = split_frontmatter(input)?;
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(frontmatter)?;
    validate(&frontmatter)?;
    let body = SkillBody::parse(body)?;
    Ok(SkillDoc { frontmatter, body })
}

fn split_frontmatter(input: &str) -> Result<(&str, &str), ParseError> {
    let (input, separator) = if let Some(input) = input.strip_prefix("---\n") {
        (input, "\n---\n")
    } else if let Some(input) = input.strip_prefix("---\r\n") {
        (input, "\r\n---\r\n")
    } else {
        return Err(ParseError::MissingFrontmatter);
    };
    let (frontmatter, body) = input
        .split_once(separator)
        .ok_or(ParseError::UnclosedFrontmatter)?;
    Ok((frontmatter, body))
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("skill document must start with YAML frontmatter")]
    MissingFrontmatter,
    #[error("frontmatter must be closed with ---")]
    UnclosedFrontmatter,
    #[error("invalid frontmatter: {0}")]
    Frontmatter(#[from] serde_yaml::Error),
    #[error("missing required body section: {0}")]
    MissingSection(&'static str),
    #[error("unexpected body section: {0}")]
    UnexpectedSection(String),
    #[error("duplicate body section: {0}")]
    DuplicateSection(&'static str),
    #[error("validation error: {0}")]
    Validation(String),
}
