use crate::projector::TargetRuntime;

pub(crate) fn yaml_scalar(value: &str) -> String {
    serde_yaml::to_string(value)
        .expect("serializing scalar cannot fail")
        .trim()
        .to_string()
}

pub(crate) fn yaml_double_quoted(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
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
