# Eval: Test Instance Live-Session Proof

Document Class: Eval
Status: Accepted
Date: 2026-06-22
Category: Developer runtime isolation, install proof
Scope: Phase 6 proof for `wiki/plans/test-instance-namespaced-binary.plan.md`
after `LLM_WIKI_INSTANCE=test` implementation and review fixes.
Sources: `tools/test-instance-live-session-proof.sh`;
`target/test-instance-live-session/report.md`;
`target/test-instance-live-session/managed-status.txt`;
`wiki/plans/test-instance-namespaced-binary.plan.md`.

## Result

The snapshot-bracketed test-instance proof passed on 2026-06-22.

Command:

```text
rtk just test-instance-live-session-proof
```

Evidence:

- Production pre-snapshot hash:
  `6300c89979f24d77f8044b9fc74e1849cfaefd4beb38c757b38089ad59382d5b`.
- Production post-snapshot hash:
  `6300c89979f24d77f8044b9fc74e1849cfaefd4beb38c757b38089ad59382d5b`.
- Managed test binary status reported `llm-wiki-test 0.2.1`, installed by
  `llm-wiki-test`, with managed binary
  `/Users/nicolasmartino/.llm_wiki-test/bin/llm-wiki-test`.
- The proof installed test-instance Claude/Codex skills, verified managed-binary
  paths and suffixed skill references, ran the managed `build --target both`
  output check, uninstalled the test instance, and verified no `wiki*-test`
  skill directories or `.llm_wiki-test` managed home remained.
- The debug carrier was restored to production identity after the run
  (`Usage: llm-wiki [OPTIONS] <COMMAND>`).

## Follow-Up Fix

The first Phase 6 attempts exposed that `uninstall` removed manifest-owned skill
files but left empty skill directories. `src/uninstall.rs` now prunes empty
manifest-owned skill directories after file removal, and
`tests/install.rs::uninstall_removes_manifest_owned_files_only` asserts the
directory is removed while unrelated user skill directories remain.

## Limits

This eval closes the test-instance coexistence proof. It does not provide native
Linux proof, Windows proof, or real GGUF release proof.
