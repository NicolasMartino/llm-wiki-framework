use anyhow::{Context, Result};
use askama::Template;
use chrono::Utc;

use super::profile::ProjectProfile;

#[derive(Template)]
#[template(path = "base/project_guidelines.md", escape = "none")]
struct ProjectGuidelinesTemplate<'a> {
    project_name: &'a str,
    project_description: &'a str,
    date: &'a str,
    include_ml_ai: bool,
    include_qmd: bool,
    guidelines_fragments: &'a [String],
}

#[derive(Template)]
#[template(path = "base/agents.md", escape = "none")]
struct AgentsTemplate<'a> {
    project_name: &'a str,
    project_description: &'a str,
    include_ml_ai: bool,
    agents_fragments: &'a [String],
}

pub fn render_project_guidelines_with_fragments(
    name: &str,
    description: &str,
    profile: &ProjectProfile,
    guidelines_fragments: &[String],
) -> Result<String> {
    let date = Utc::now().date_naive().to_string();
    let template = ProjectGuidelinesTemplate {
        project_name: name,
        project_description: description,
        date: &date,
        include_ml_ai: profile.include_ml_ai,
        include_qmd: profile.include_qmd,
        guidelines_fragments,
    };
    template
        .render()
        .map(|rendered| compact_blank_lines(&rendered))
        .context("failed to render project_guidelines.md")
}

pub fn render_agent_template_with_fragments(
    name: &str,
    description: &str,
    profile: &ProjectProfile,
    agents_fragments: &[String],
) -> Result<String> {
    let template = AgentsTemplate {
        project_name: name,
        project_description: description,
        include_ml_ai: profile.include_ml_ai,
        agents_fragments,
    };
    template
        .render()
        .map(|rendered| compact_blank_lines(&rendered))
        .context("failed to render AGENTS.md")
}

fn compact_blank_lines(input: &str) -> String {
    let mut output = String::new();
    let mut blank_count = 0;
    let mut in_fenced_block = false;

    for line in input.lines() {
        if line.trim_start().starts_with("```") {
            in_fenced_block = !in_fenced_block;
            blank_count = 0;
            output.push_str(line.trim_end());
            output.push('\n');
            continue;
        }

        if line.trim().is_empty() {
            if in_fenced_block {
                output.push('\n');
                continue;
            }
            blank_count += 1;
            if blank_count > 1 {
                continue;
            }
        } else {
            blank_count = 0;
        }
        output.push_str(line.trim_end());
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::compact_blank_lines;

    #[test]
    fn compact_blank_lines_preserves_fenced_block_spacing() {
        let input = "Before\n\n\n```text\nalpha\n\nbeta\n```\n\n\nAfter\n";

        assert_eq!(
            compact_blank_lines(input),
            "Before\n\n```text\nalpha\n\nbeta\n```\n\nAfter\n"
        );
    }
}
