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

        fn set_once(
            target: &mut Option<String>,
            section: &'static str,
            value: String,
        ) -> Result<(), ParseError> {
            if target.is_some() {
                return Err(ParseError::DuplicateSection(section));
            }
            *target = Some(value);
            Ok(())
        }

        fn flush(
            current: Option<&str>,
            buffer: &mut String,
            title: &mut Option<String>,
            purpose: &mut Option<String>,
            behavior: &mut Option<String>,
            invocation: &mut Option<String>,
            notes: &mut Option<String>,
        ) -> Result<(), ParseError> {
            let Some(section) = current else {
                buffer.clear();
                return Ok(());
            };
            let value = buffer.trim().to_string();
            match section {
                "title" => set_once(title, "title", value)?,
                "purpose" => set_once(purpose, "Purpose", value)?,
                "behavior" => set_once(behavior, "Behavior", value)?,
                "invocation" => set_once(invocation, "Invocation", value)?,
                "notes" => set_once(notes, "Notes", value)?,
                _ => {}
            }
            buffer.clear();
            Ok(())
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
                )?;
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
                )?;
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
        )?;

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
