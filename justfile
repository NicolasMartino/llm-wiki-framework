set shell := ["bash", "-cu"]

default:
    @just --list

fmt:
    cargo fmt --all --check

fmt-fix:
    cargo fmt --all

# A plain test run, for people. `just verify` runs the suite through
# `snapshots` instead, which runs every workspace test once.
test: test-tools
    cargo test --workspace

# tools/release-e2e is a workspace of its own, so `--workspace` misses it.
test-tools:
    cargo test --manifest-path tools/release-e2e/Cargo.toml

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

clippy-strict:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -D dead_code

# The test suite, run once: `cargo insta test --check` runs every workspace
# test and fails on a snapshot that does not match, so the tests and the
# snapshot check are one run.
snapshots:
    @cargo insta --version >/dev/null 2>&1 || { echo "snapshots needs cargo-insta: install it with \`cargo install cargo-insta\`" >&2; exit 1; }
    cargo insta test --workspace --check

# `snapshots` under coverage instrumentation, then the coverage gate over that
# same run, so the full CI runs the suite once. The instrumented build goes to
# target/llvm-cov-target, as `cargo llvm-cov` puts it, and leaves target/debug
# alone.
coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo insta --version >/dev/null 2>&1 || { echo "coverage needs cargo-insta: install it with \`cargo install cargo-insta\`" >&2; exit 1; }
    cargo llvm-cov --version >/dev/null 2>&1 || { echo "coverage needs cargo-llvm-cov: install it with \`cargo install cargo-llvm-cov --locked\`" >&2; exit 1; }
    export CARGO_TARGET_DIR="{{ justfile_directory() }}/target/llvm-cov-target"
    llvm_cov_env=$(cargo llvm-cov show-env --sh)
    eval "$llvm_cov_env"
    cargo llvm-cov clean --workspace
    cargo insta test --workspace --check
    cargo llvm-cov report --workspace --fail-under-lines 80

# The dated nightly is named once, in tools/udeps-nightly, which the strict
# gates and CI's unused-dependencies job also read.
udeps:
    cargo +"$(tr -d '[:space:]' < tools/udeps-nightly)" udeps --workspace

# The strictest gates over the strict crates (llm-wiki-core and poman); see
# tools/strict-gates.sh. Name gates to run only those: `just strict fmt tests`.
strict *gates:
    tools/strict-gates.sh {{gates}}

# Passes only when rg finds nothing (exit 1): a bare `! rg` would also pass
# when rg is missing or a path is wrong.
audit-legacy:
    #!/usr/bin/env bash
    set -u
    command -v rg >/dev/null || { echo "audit-legacy needs ripgrep (rg): install it with your package manager, or \`cargo install ripgrep\`" >&2; exit 1; }
    audit() {
      rg -n "$@"
      case $? in
        1) ;;
        0) echo "audit-legacy: legacy wording found above" >&2; exit 1 ;;
        *) echo "audit-legacy: rg failed" >&2; exit 1 ;;
      esac
    }
    audit -g '!tests/fixtures/search-eval/**' -g '!tests/fixtures/page-reader/**' 'Successor:|Will Supersede On D8 Completion|<!-- CLAUDE -->|<!-- CODEX -->|<!-- END -->|skills/build\.sh|bash renderer|(?:^|[^a-z])build\.sh' wiki README.md .github src tests Cargo.toml
    audit 'legacy shell renderer|legacy skill render script|legacy render script|<!-- TAG -->' wiki/specs wiki/decisions wiki/plans wiki/roadmaps wiki/index.md README.md AGENTS.MD CLAUDE.md

# Every workspace test runs once, in `snapshots`.
verify: fmt snapshots checks

# The full CI's test job: `just verify` with its one test run under coverage
# (`coverage` in place of `snapshots`), so the suite still runs once.
verify-coverage: fmt coverage checks

# The gates `verify` and `verify-coverage` share; one list, so they cannot
# drift apart.
checks: test-tools clippy-strict audit-legacy branch-status-test local-release-test

# The integration test files that take longest; they run in `just verify` and
# the full CI, not in `just fast-check`.
slow_tests := "install mcp_install post_install properties search_commands"

# The fast check on PRs into develop: `just verify` without the slow test
# files. One `cargo insta test --check` runs the quick tests and checks the
# snapshots together. A new test file is quick until it is named above.
fast-check: fmt clippy-strict audit-legacy branch-status-test
    #!/usr/bin/env bash
    set -euo pipefail
    cargo insta --version >/dev/null 2>&1 || { echo "fast-check needs cargo-insta: install it with \`cargo install cargo-insta\`" >&2; exit 1; }
    args=()
    for file in tests/*.rs; do
      name=$(basename "$file" .rs)
      [[ " {{ slow_tests }} " == *" $name "* ]] || args+=(--test "$name")
    done
    cargo insta test --check --bins "${args[@]}"
    just test-tools

verify-full: verify-coverage udeps

post-install:
    cargo test --test post_install

# Both binaries: `llm-wiki install` takes poman from beside llm-wiki.
build-bin:
    cargo build --bin llm-wiki --bin poman

run *args:
    cargo run --bin llm-wiki -- {{args}}

# Install takes poman from beside llm-wiki, so both are built first.
install:
    cargo build --bin poman
    cargo run --bin llm-wiki -- install

install-force:
    cargo build --bin poman
    cargo run --bin llm-wiki -- install --force

uninstall:
    cargo run --bin llm-wiki -- uninstall

status:
    cargo run --bin llm-wiki -- status

doctor:
    cargo run --bin llm-wiki -- doctor

init path name type="web" scale="small" description="One sentence description.":
    cargo run --bin llm-wiki -- init "{{path}}" --non-interactive --name "{{name}}" --description "{{description}}" --type "{{type}}" --scale "{{scale}}"

release-guard:
    test -z "${LLM_WIKI_INSTANCE:-}" || { echo "refusing release command with LLM_WIKI_INSTANCE=${LLM_WIKI_INSTANCE}"; exit 1; }

# The owner's local release: builds this checkout's release archives with
# cargo-dist (no tag, nothing published), saves what is installed now under
# ~/.llm_wiki/local-release/, and installs llm-wiki and poman from the archives,
# registering both MCP servers. Flags go to `llm-wiki install`. See
# tools/local-release.sh.
local-release *args: release-guard
    tools/local-release.sh install {{args}}

# Puts back exactly what the newest `just local-release` saved, but refuses
# while a file changed after the release; `--overwrite` keeps a copy of each
# and reverts anyway.
local-release-revert *args: release-guard
    tools/local-release.sh revert {{args}}

release-plan: release-guard
    dist plan

release-build: release-guard
    dist build

release-e2e category="smoke": release-guard
    cargo build --bin llm-wiki --bin poman
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- {{category}} --artifact target/debug/llm-wiki

release-e2e-skip-infra category="smoke": release-guard
    cargo build --bin llm-wiki --bin poman
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- {{category}} --artifact target/debug/llm-wiki --skip-infra

release-e2e-gguf *args: release-guard
    cargo build --bin llm-wiki --bin poman
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- gguf --artifact target/debug/llm-wiki --manual-models {{args}}

release-e2e-native-linux-archive archive checksum target_triple="x86_64-unknown-linux-gnu" output_dir="target/release-e2e-native-linux-amd64": release-guard
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- search --archive "{{archive}}" --checksum "{{checksum}}" --target-triple "{{target_triple}}" --output-dir "{{output_dir}}"

release-e2e-native-linux-dist-build target_triple="x86_64-unknown-linux-gnu" target_dir="target/release-e2e-native-linux-amd64-dist": release-guard
    mkdir -p "{{target_dir}}"
    env CARGO_TARGET_DIR="{{target_dir}}" dist build --artifacts=local --target "{{target_triple}}" --output-format=json > "{{target_dir}}/dist-manifest.json"
    file "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz" "{{target_dir}}/distrib/poman-{{target_triple}}.tar.xz"

release-e2e-native-linux-dist-build-and-test target_triple="x86_64-unknown-linux-gnu" target_dir="target/release-e2e-native-linux-amd64-dist" output_dir="target/release-e2e-native-linux-amd64": release-guard
    just release-e2e-native-linux-dist-build "{{target_triple}}" "{{target_dir}}"
    just release-e2e-native-linux-archive "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz" "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz.sha256" "{{target_triple}}" "{{output_dir}}"

release-e2e-linux artifact target_triple="aarch64-unknown-linux-gnu" platform="linux/arm64" image="debian:bookworm-slim" output_dir="target/release-e2e": release-guard
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- linux --artifact "{{artifact}}" --target-triple "{{target_triple}}" --docker-platform "{{platform}}" --docker-image "{{image}}" --output-dir "{{output_dir}}"

release-e2e-linux-archive archive checksum target_triple="aarch64-unknown-linux-gnu" platform="linux/arm64" image="debian:bookworm-slim" output_dir="target/release-e2e": release-guard
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- linux --archive "{{archive}}" --checksum "{{checksum}}" --target-triple "{{target_triple}}" --docker-platform "{{platform}}" --docker-image "{{image}}" --output-dir "{{output_dir}}"

release-e2e-linux-skip-infra artifact target_triple="aarch64-unknown-linux-gnu" platform="linux/arm64" image="debian:bookworm-slim" network="none" output_dir="target/release-e2e": release-guard
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- linux --artifact "{{artifact}}" --target-triple "{{target_triple}}" --docker-platform "{{platform}}" --docker-image "{{image}}" --docker-network "{{network}}" --skip-infra --output-dir "{{output_dir}}"

release-e2e-linux-build platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_dir="target/release-e2e-linux-aarch64": release-guard
    mkdir -p target/release-e2e-docker-home target/release-e2e-cargo-home "{{target_dir}}"
    docker build --platform "{{platform}}" -t "{{image}}" -f infra/release-e2e/linux-builder.Dockerfile infra/release-e2e
    docker run --rm --platform "{{platform}}" --user "$(id -u):$(id -g)" -e HOME=/work/target/release-e2e-docker-home -e CARGO_HOME=/work/target/release-e2e-cargo-home -e CARGO_TARGET_DIR=/work/{{target_dir}} -v "{{justfile_directory()}}:/work" -w /work "{{image}}" cargo build --bin llm-wiki --bin poman --release
    file "{{target_dir}}/release/llm-wiki"

release-e2e-linux-build-and-test platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_dir="target/release-e2e-linux-aarch64" artifact_image="debian:bookworm-slim" target_triple="aarch64-unknown-linux-gnu" output_dir="target/release-e2e": release-guard
    just release-e2e-linux-build "{{platform}}" "{{image}}" "{{target_dir}}"
    just release-e2e-linux "{{target_dir}}/release/llm-wiki" "{{target_triple}}" "{{platform}}" "{{artifact_image}}" "{{output_dir}}"

release-e2e-linux-dist-build platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_triple="aarch64-unknown-linux-gnu" target_dir="target/release-e2e-linux-dist-aarch64":
    mkdir -p target/release-e2e-docker-home target/release-e2e-cargo-home "{{target_dir}}"
    docker build --platform "{{platform}}" -t "{{image}}" -f infra/release-e2e/linux-builder.Dockerfile infra/release-e2e
    docker run --rm --platform "{{platform}}" --user "$(id -u):$(id -g)" -e HOME=/work/target/release-e2e-docker-home -e CARGO_HOME=/work/target/release-e2e-cargo-home -e CARGO_TARGET_DIR=/work/{{target_dir}} -v "{{justfile_directory()}}:/work" -w /work "{{image}}" sh -c "PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin dist build --artifacts=local --target '{{target_triple}}' --output-format=json > '/work/{{target_dir}}/dist-manifest.json'"
    file "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz" "{{target_dir}}/distrib/poman-{{target_triple}}.tar.xz"

release-e2e-linux-dist-build-and-test platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_triple="aarch64-unknown-linux-gnu" target_dir="target/release-e2e-linux-dist-aarch64" artifact_image="debian:bookworm-slim" output_dir="target/release-e2e":
    just release-e2e-linux-dist-build "{{platform}}" "{{image}}" "{{target_triple}}" "{{target_dir}}"
    just release-e2e-linux-archive "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz" "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz.sha256" "{{target_triple}}" "{{platform}}" "{{artifact_image}}" "{{output_dir}}"

test-instance-live-session-proof:
    bash tools/test-instance-live-session-proof.sh

release-e2e-linux-infra-up stack="e2e":
    cd infra/release-e2e && pulumi stack select "{{stack}}" --create && pulumi up --yes --parallel 1

release-e2e-linux-infra-down stack="e2e":
    cd infra/release-e2e && pulumi stack select "{{stack}}" && pulumi destroy --yes

git-summary:
    git status --short --branch
    git log --oneline --decorate --max-count=14

# `just branch-status` against a scratch origin; `just verify` runs it.
branch-status-test:
    bash tools/branch-status-test.sh

# The local release's save and revert under a temporary HOME; `just verify`
# runs it.
local-release-test:
    bash tools/local-release-test.sh

# The commit on develop where statuses moved into PRs (#32, 2026-10-06). A plan
# that already named its branch there keeps the line until its own PR completes
# it; tools/branch-status-test.sh sets its own.
statuses_moved_into_prs := "816df9b6b1f87c470d6dd58578f7aa399c773899"

# See wiki/decisions/work-in-flight-is-a-pushed-branch.decision.md.
# What is in flight, and whether the plans agree with origin.
branch-status:
    #!/usr/bin/env bash
    set -uo pipefail
    cd "{{ justfile_directory() }}"
    ok=0
    say() { printf '  %-9s %-30s %s\n' "$1" "$2" "$3"; case "$1" in STALE) ok=1 ;; esac; return 0; }

    # `origin` is the authority, not `git worktree list`, which one machine alone
    # sees. A plan names its branch on that branch (the worker's first push), so
    # each branch's own plans are read from its remote-tracking ref, fresh from
    # a fetch. The fetch never prompts: origin is HTTPS, and gh answers for it.
    # The refspec is explicit so a single-branch clone still sees every branch.
    if GIT_TERMINAL_PROMPT=0 GIT_SSH_COMMAND="ssh -o BatchMode=yes" \
         git fetch --quiet --prune origin '+refs/heads/*:refs/remotes/origin/*' 2>/dev/null; then
      origin_note="fetched"
    else
      origin_note="fetch failed: refs from the last fetch"
    fi
    remotes=$(git for-each-ref --format='%(refname:lstrip=3)' refs/remotes/origin | grep -vx HEAD)
    on_origin() { printf '%s\n' "$remotes" | grep -qxF -- "$1"; }
    # The branch this checkout is on; detached (as a PR checkout for review
    # leaves it), the origin branches that point at HEAD.
    here=$(git symbolic-ref --quiet --short HEAD \
      || git for-each-ref --points-at HEAD --format='%(refname:lstrip=3)' refs/remotes/origin | grep -vx HEAD)
    is_here() { printf '%s\n' "$here" | grep -qxF -- "$1"; }

    plan_status() { awk -F': ' '/^- Status:/{print $2; exit}'; }
    plan_branch() { awk -F'`' '/^- Branch: /{print $2; exit}' "$1"; }
    plans() { find wiki/plans -maxdepth 1 -name '*.plan.md' 2>/dev/null; }

    # The plans on this checkout that name a branch, as "<plan>\t<branch>".
    checkout_named=$(while IFS= read -r f; do
      b=$(plan_branch "$f"); [ -n "$b" ] && printf '%s\t%s\n' "$f" "$b"
    done < <(plans))

    # The plans a branch's own ref names it in, as "<plan>\t<status there>".
    plans_on_ref() {
      local branch="$1" ref="origin/$1" f
      git grep -l -F -e "- Branch: \`$branch\`" "$ref" -- 'wiki/plans/*.plan.md' 2>/dev/null \
        | while IFS= read -r f; do
            f=${f#"$ref:"}
            printf '%s\t%s\n' "$(basename "$f")" "$(git show "$ref:$f" | plan_status)"
          done
    }

    # Whether a plan already named this branch on develop when statuses moved
    # into PRs. Only those lines may stay on develop until their PR merges.
    named_before_prs() {
      git show "{{ statuses_moved_into_prs }}:$1" 2>/dev/null | grep -qxF -- "- Branch: \`$2\`"
    }

    echo "branches on origin ($origin_note)"
    while IFS= read -r branch; do
      [ -n "$branch" ] || continue
      [ "$branch" = "develop" ] && { say ok develop "the base"; continue; }
      [ "$branch" = "master" ] && { say ok master "takes develop through a PR"; continue; }
      hits=$(plans_on_ref "$branch")
      old=$(printf '%s\n' "$checkout_named" | awk -F'\t' -v b="$branch" '$2 == b {print $1}' \
        | while IFS= read -r f; do named_before_prs "$f" "$branch" && echo "$f"; done)
      if [ -n "$hits" ]; then
        # A plan whose work is under way is Active or Blocked; any other status
        # with its Branch line still in would carry a stale line into develop.
        while IFS=$'\t' read -r plan st; do
          case "$st" in
            Active|Blocked) say ok "$branch" "-> $plan [$st]" ;;
            *) say STALE "$branch" "-> $plan [$st]; remove its Branch line" ;;
          esac
        done <<< "$hits"
      elif [ -n "$old" ]; then
        say ok "$branch" "-> $(printf '%s\n' "$old" | xargs -n1 basename | paste -sd, -), its Branch line on this checkout (see below)"
      else
        say "no plan" "$branch" "answers to an issue, or its plan lacks a Branch: line (see the board)"
      fi
    done <<< "$remotes"

    echo ""
    # A Branch line lives on its own branch and leaves with the merge, so a plan
    # on this checkout naming another branch is left over, with one exception:
    # a line already on develop when statuses moved into PRs (2026-10-06, the
    # commit above) stays until its own PR completes the plan. Any other line is
    # STALE, whether or not its branch is still on origin: merged branches stay.
    echo "plans on this checkout naming a branch"
    while IFS=$'\t' read -r plan branch; do
      [ -n "$plan" ] || continue
      name=$(basename "$plan"); st=$(plan_status < "$plan")
      case "$st" in
        Active|Blocked) ;;
        *) say STALE "$name" "$st, but names $branch; remove the Branch line"; continue ;;
      esac
      if is_here "$branch"; then
        say ok "$name" "$branch [$st], this branch's own plan"
      elif ! named_before_prs "$plan" "$branch"; then
        say STALE "$name" "names $branch on this checkout; a Branch line lives on its own branch"
      elif on_origin "$branch"; then
        say "in flight" "$name" "$branch [$st], set before statuses moved into PRs"
      else
        say STALE "$name" "names $branch, which origin does not have; the work merged or never was pushed"
      fi
    done <<< "$checkout_named"

    echo ""
    # Informational: a worktree is a convenience on one machine, and its branch
    # counts as in flight only once it is pushed. An unpushed branch is work no
    # other machine can see or continue.
    echo "worktrees on this machine"
    git worktree list --porcelain | awk '
      /^worktree /   { w = substr($0, 10); b = "(detached)" }
      /^branch /     { b = substr($0, 8); sub("refs/heads/", "", b) }
      /^detached$/   { b = "(detached)" }
      /^$/           { if (w != "") { print w "\t" b; w = "" } }
      END            { if (w != "") print w "\t" b }' \
    | while IFS=$'\t' read -r dir branch; do
        name=$(basename "$dir")
        case "$branch" in
          "(detached)") say ok "$name" "detached HEAD" ;;
          *) on_origin "$branch" \
               && say ok "$name" "$branch, pushed" \
               || say local "$name" "$branch is not on origin yet" ;;
        esac
      done

    echo ""
    echo "status vocabulary"
    # `templates/base/project_guidelines.md` governs: Draft, Active, Blocked,
    # Completed, Superseded. A plan completed from now on also says where its
    # proof holds, because "completed" alone is the question people ask:
    # `(develop)` once its PR merges into develop. `(master)` stays on plans
    # marked so before work moved to develop.
    while IFS= read -r plan; do
      st=$(plan_status < "$plan")
      case "$st" in
        Draft|Active|Blocked|Superseded) ;;
        "Completed (local)"|"Completed (develop)"|"Completed (master)"|"Completed (spike)") ;;
        # Plans completed before 2026-10-06 say only "Completed".
        Completed) ;;
        *) say STALE "$(basename "$plan")" "not a plan status: '$st'" ;;
      esac
    done < <(plans)

    echo ""
    [ "$ok" = 0 ] && echo "plans and origin agree." || echo "Fix the plans named above."
    exit "$ok"
