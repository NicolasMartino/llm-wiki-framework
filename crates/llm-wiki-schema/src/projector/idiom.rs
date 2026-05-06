use crate::frontmatter::Runtime;
use crate::projector::TargetRuntime;

pub fn rewrite_invocation(input: &str, skill_name: &str, runtime: TargetRuntime) -> String {
    let replacement = match runtime {
        TargetRuntime::Claude => format!("/{skill_name}"),
        TargetRuntime::Codex => format!("${skill_name}"),
    };
    input.replace(&format!("<{skill_name}>"), &replacement)
}

pub(crate) fn supports_runtime(runtimes: &[Runtime], runtime: TargetRuntime) -> bool {
    match runtime {
        TargetRuntime::Claude => runtimes.contains(&Runtime::Claude),
        TargetRuntime::Codex => runtimes.contains(&Runtime::Codex),
    }
}
