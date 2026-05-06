# Skills Source

This directory contains the canonical source of framework skills.

Edit `assets/skills/<name>/SKILL.md`, then regenerate runtime-specific outputs
with the `llm-wiki` binary:

```bash
cargo run -- build --out .
```

Generated outputs are committed for first-clone usability:

- `.claude/skills/<name>/SKILL.md`
- `.codex/skills/<name>/SKILL.md`
- `.codex/skills/<name>/agents/openai.yaml`

Canonical skills use typed YAML frontmatter plus fixed markdown sections. The
Rust projector owns runtime-specific invocation idioms.
