# Skills Source

This directory is the canonical source of framework skills.

Edit `skills/<name>/SKILL.md`, then regenerate runtime-specific outputs:

```bash
bash skills/build.sh
```

Generated outputs are committed for first-clone usability:

- `.claude/skills/<name>/SKILL.md`
- `.codex/skills/<name>/SKILL.md`
- `.codex/skills/<name>/agents/openai.yaml`

Conditional blocks:

- `<!-- CLAUDE --> ... <!-- END -->` is kept only in Claude output.
- `<!-- CODEX --> ... <!-- END -->` is kept only in Codex output.
- Text outside conditional blocks is shared by both runtimes.

The `knowledge` skill is Codex-only because it implements the `$knowledge`
dispatcher namespace.
