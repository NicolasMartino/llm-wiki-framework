use crate::projector::TargetRuntime;

pub(crate) fn render_frontmatter(name: &str, description: &str) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n",
        yaml_scalar(name),
        yaml_scalar(description)
    )
}

fn yaml_scalar(value: &str) -> String {
    serde_yaml::to_string(value)
        .expect("serializing scalar cannot fail")
        .trim()
        .to_string()
}

pub(crate) fn description_for(description: &str, runtime: TargetRuntime) -> String {
    let (runtime_name, verb) = match runtime {
        TargetRuntime::Claude => ("Claude Code", "invoke"),
        TargetRuntime::Codex => ("Codex", "use"),
    };
    description
        .replace("{runtime}", runtime_name)
        .replace("{verb}", verb)
}
