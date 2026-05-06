use chrono::Utc;

use super::profile::ProjectProfile;

pub fn render_project_guidelines(
    template: &str,
    name: &str,
    description: &str,
    profile: &ProjectProfile,
) -> String {
    let date = Utc::now().date_naive().to_string();
    let replaced = template
        .replace("{{PROJECT_NAME}}", name)
        .replace("{{PROJECT_DESCRIPTION}}", description)
        .replace("{{DATE}}", &date);
    resolve_conditionals(&replaced, profile)
}

pub fn render_agent_template(
    template: &str,
    name: &str,
    description: &str,
    profile: &ProjectProfile,
) -> String {
    template
        .replace("{{PROJECT_NAME}}", name)
        .replace("{{PROJECT_DESCRIPTION}}", description)
        .replace(
            "{{ML_AI_TYPES}}",
            if profile.include_ml_ai {
                ", experiment, eval"
            } else {
                ""
            },
        )
}

fn resolve_conditionals(input: &str, profile: &ProjectProfile) -> String {
    let mut output = Vec::new();
    let mut skip_until: Option<&str> = None;
    for line in input.lines() {
        if let Some(section) = skip_until {
            if line.contains(&format!("<!-- END:{section} -->")) {
                skip_until = None;
            }
            continue;
        }

        if line.contains("<!-- SECTION:ML_AI") {
            if profile.include_ml_ai {
                continue;
            }
            skip_until = Some("ML_AI");
            continue;
        }
        if line.contains("<!-- SECTION:QMD") {
            if profile.include_qmd {
                continue;
            }
            skip_until = Some("QMD");
            continue;
        }
        if line.contains("<!-- END:ML_AI -->") || line.contains("<!-- END:QMD -->") {
            continue;
        }
        if line.contains("<!-- CONDITIONAL:ML_AI -->") {
            if profile.include_ml_ai {
                output.push(line.replace("<!-- CONDITIONAL:ML_AI -->", ""));
            }
            continue;
        }
        if line.trim_start().starts_with("<!--") && line.trim_end().ends_with("-->") {
            continue;
        }
        output.push(line.to_string());
    }
    compact_blank_lines(&output.join("\n"))
}

fn compact_blank_lines(input: &str) -> String {
    let mut output = String::new();
    let mut blank_count = 0;
    for line in input.lines() {
        if line.trim().is_empty() {
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
