use crate::doc::ParseError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillBody {
    pub title: String,
    pub purpose: String,
    pub behavior: String,
    pub invocation: String,
    pub notes: Option<String>,
}

impl SkillBody {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let mut title = None;
        let mut purpose = None;
        let mut behavior = None;
        let mut invocation = None;
        let mut notes = None;
        let mut current: Option<&str> = None;
        let mut buffer = String::new();

        fn flush(
            current: Option<&str>,
            buffer: &mut String,
            title: &mut Option<String>,
            purpose: &mut Option<String>,
            behavior: &mut Option<String>,
            invocation: &mut Option<String>,
            notes: &mut Option<String>,
        ) {
            let Some(section) = current else {
                buffer.clear();
                return;
            };
            let value = buffer.trim().to_string();
            match section {
                "title" => *title = Some(value),
                "purpose" => *purpose = Some(value),
                "behavior" => *behavior = Some(value),
                "invocation" => *invocation = Some(value),
                "notes" => *notes = Some(value),
                _ => {}
            }
            buffer.clear();
        }

        for line in input.lines() {
            if let Some(rest) = line.strip_prefix("# ") {
                flush(
                    current,
                    &mut buffer,
                    &mut title,
                    &mut purpose,
                    &mut behavior,
                    &mut invocation,
                    &mut notes,
                );
                current = Some("title");
                buffer.push_str(rest);
                buffer.push('\n');
            } else if let Some(rest) = line.strip_prefix("## ") {
                flush(
                    current,
                    &mut buffer,
                    &mut title,
                    &mut purpose,
                    &mut behavior,
                    &mut invocation,
                    &mut notes,
                );
                current = match rest.trim() {
                    "Purpose" => Some("purpose"),
                    "Behavior" => Some("behavior"),
                    "Invocation" => Some("invocation"),
                    "Notes" => Some("notes"),
                    heading => return Err(ParseError::UnexpectedSection(heading.to_string())),
                };
            } else if current.is_some() {
                buffer.push_str(line);
                buffer.push('\n');
            }
        }

        flush(
            current,
            &mut buffer,
            &mut title,
            &mut purpose,
            &mut behavior,
            &mut invocation,
            &mut notes,
        );

        Ok(Self {
            title: required("title", title)?,
            purpose: required("Purpose", purpose)?,
            behavior: required("Behavior", behavior)?,
            invocation: required("Invocation", invocation)?,
            notes: notes.filter(|value| !value.is_empty()),
        })
    }
}

fn required(section: &'static str, value: Option<String>) -> Result<String, ParseError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ParseError::MissingSection(section)),
    }
}
