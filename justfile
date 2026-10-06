set shell := ["bash", "-cu"]

default:
    @just --list

fmt:
    cargo fmt --all --check

fmt-fix:
    cargo fmt --all

test:
    cargo test --workspace
    cargo test --manifest-path tools/release-e2e/Cargo.toml

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

clippy-strict:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -D dead_code

snapshots:
    @cargo insta --version >/dev/null 2>&1 || { echo "snapshots needs cargo-insta: install it with \`cargo install cargo-insta\`" >&2; exit 1; }
    cargo insta test --workspace --check

coverage:
    cargo llvm-cov --workspace --fail-under-lines 80

udeps:
    cargo +nightly udeps --workspace

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
    audit 'Successor:|Will Supersede On D8 Completion|<!-- CLAUDE -->|<!-- CODEX -->|<!-- END -->|skills/build\.sh|bash renderer|(?:^|[^a-z])build\.sh' wiki README.md .github src tests Cargo.toml
    audit 'legacy shell renderer|legacy skill render script|legacy render script|<!-- TAG -->' wiki/specs wiki/decisions wiki/plans wiki/roadmaps wiki/index.md README.md AGENTS.MD CLAUDE.md

verify: fmt test clippy-strict snapshots audit-legacy

# The integration test files that take longest; they run in `just verify` and
# the full CI, not in `just fast-check`.
slow_tests := "install mcp_install post_install properties search_commands"

# The fast check on PRs into develop: `just verify` without the slow test
# files. One `cargo insta test --check` runs the quick tests and checks the
# snapshots together. A new test file is quick until it is named above.
fast-check: fmt clippy-strict audit-legacy
    #!/usr/bin/env bash
    set -euo pipefail
    cargo insta --version >/dev/null 2>&1 || { echo "fast-check needs cargo-insta: install it with \`cargo install cargo-insta\`" >&2; exit 1; }
    args=()
    for file in tests/*.rs; do
      name=$(basename "$file" .rs)
      [[ " {{ slow_tests }} " == *" $name "* ]] || args+=(--test "$name")
    done
    cargo insta test --check --bins "${args[@]}"
    cargo test --manifest-path tools/release-e2e/Cargo.toml

verify-full: verify coverage udeps

post-install:
    cargo test --test post_install

build-bin:
    cargo build

run *args:
    cargo run -- {{args}}

build-skills:
    cargo run -- build --out .

build-skills-to out:
    cargo run -- build --out "{{out}}"

install:
    cargo run -- install

install-force:
    cargo run -- install --force

uninstall:
    cargo run -- uninstall

status:
    cargo run -- status

doctor:
    cargo run -- doctor

init path name type="web" scale="small" description="One sentence description.":
    cargo run -- init "{{path}}" --non-interactive --name "{{name}}" --description "{{description}}" --type "{{type}}" --scale "{{scale}}"

release-guard:
    test -z "${LLM_WIKI_INSTANCE:-}" || { echo "refusing release command with LLM_WIKI_INSTANCE=${LLM_WIKI_INSTANCE}"; exit 1; }

release-plan: release-guard
    dist plan

release-build: release-guard
    dist build

release-e2e category="smoke": release-guard
    cargo build --bin llm-wiki
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- {{category}} --artifact target/debug/llm-wiki

release-e2e-skip-infra category="smoke": release-guard
    cargo build --bin llm-wiki
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- {{category}} --artifact target/debug/llm-wiki --skip-infra

release-e2e-gguf *args: release-guard
    cargo build --bin llm-wiki
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- gguf --artifact target/debug/llm-wiki --manual-models {{args}}

release-e2e-native-linux-archive archive checksum target_triple="x86_64-unknown-linux-gnu" output_dir="target/release-e2e-native-linux-amd64": release-guard
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- search --archive "{{archive}}" --checksum "{{checksum}}" --target-triple "{{target_triple}}" --output-dir "{{output_dir}}"

release-e2e-native-linux-dist-build target_triple="x86_64-unknown-linux-gnu" target_dir="target/release-e2e-native-linux-amd64-dist": release-guard
    mkdir -p "{{target_dir}}"
    env CARGO_TARGET_DIR="{{target_dir}}" dist build --artifacts=local --target "{{target_triple}}" --output-format=json > "{{target_dir}}/dist-manifest.json"
    file "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz"

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
    docker run --rm --platform "{{platform}}" --user "$(id -u):$(id -g)" -e HOME=/work/target/release-e2e-docker-home -e CARGO_HOME=/work/target/release-e2e-cargo-home -e CARGO_TARGET_DIR=/work/{{target_dir}} -v "{{justfile_directory()}}:/work" -w /work "{{image}}" cargo build --bin llm-wiki --release
    file "{{target_dir}}/release/llm-wiki"

release-e2e-linux-build-and-test platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_dir="target/release-e2e-linux-aarch64" artifact_image="debian:bookworm-slim" target_triple="aarch64-unknown-linux-gnu" output_dir="target/release-e2e": release-guard
    just release-e2e-linux-build "{{platform}}" "{{image}}" "{{target_dir}}"
    just release-e2e-linux "{{target_dir}}/release/llm-wiki" "{{target_triple}}" "{{platform}}" "{{artifact_image}}" "{{output_dir}}"

release-e2e-linux-dist-build platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_triple="aarch64-unknown-linux-gnu" target_dir="target/release-e2e-linux-dist-aarch64":
    mkdir -p target/release-e2e-docker-home target/release-e2e-cargo-home "{{target_dir}}"
    docker build --platform "{{platform}}" -t "{{image}}" -f infra/release-e2e/linux-builder.Dockerfile infra/release-e2e
    docker run --rm --platform "{{platform}}" --user "$(id -u):$(id -g)" -e HOME=/work/target/release-e2e-docker-home -e CARGO_HOME=/work/target/release-e2e-cargo-home -e CARGO_TARGET_DIR=/work/{{target_dir}} -v "{{justfile_directory()}}:/work" -w /work "{{image}}" sh -c "PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin dist build --artifacts=local --target '{{target_triple}}' --output-format=json > '/work/{{target_dir}}/dist-manifest.json'"
    file "{{target_dir}}/distrib/llm-wiki-rs-{{target_triple}}.tar.xz"

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

# See wiki/decisions/work-in-flight-is-a-pushed-branch.decision.md.
# What is in flight, and whether the plans agree with origin.
branch-status:
    #!/usr/bin/env bash
    set -uo pipefail
    cd "{{ justfile_directory() }}"
    ok=0
    say() { printf '  %-9s %-30s %s\n' "$1" "$2" "$3"; case "$1" in STALE|UNPUSHED) ok=1 ;; esac; return 0; }

    # `origin` is the authority, not `git worktree list`, which one machine alone sees; see
    # `wiki/decisions/work-in-flight-is-a-pushed-branch.decision.md`.
    remotes=$(mktemp); trap 'rm -f "$remotes"' EXIT
    if out=$(GIT_SSH_COMMAND="ssh -o BatchMode=yes" git ls-remote --heads origin 2>/dev/null); then
      [ -z "$out" ] || printf '%s\n' "$out" | sed 's#.*refs/heads/##' > "$remotes"
      origin_note="live"
    # The remote is SSH; without the key's passphrase, ask GitHub through gh.
    elif out=$(gh api "repos/{owner}/{repo}/branches" --paginate --jq '.[].name' 2>/dev/null); then
      printf '%s\n' "$out" > "$remotes"
      origin_note="live, through gh"
    else
      git for-each-ref --format='%(refname:lstrip=3)' refs/remotes/origin \
        | grep -vx HEAD > "$remotes"
      origin_note="unreachable, using refs from the last fetch"
    fi
    on_origin() { grep -qx "$1" "$remotes"; }

    plan_status() { awk -F': ' '/^- Status:/{print $2; exit}' "$1"; }
    plan_branch() { awk -F'`' '/^- Branch: /{print $2; exit}' "$1"; }
    plans() { find wiki/plans -maxdepth 1 -name '*.plan.md' 2>/dev/null; }

    # A plan claiming a branch must also be live. Matching `Branch:` alone would
    # accept a Draft plan and report it as in flight.
    live_plan_for() {
      local want="$1" f st
      while IFS= read -r f; do
        [ "$(plan_branch "$f")" = "$want" ] || continue
        st=$(plan_status "$f")
        case "$st" in Active|Blocked) printf '%s\t%s\n' "$f" "$st"; return 0 ;; esac
      done < <(plans)
      return 1
    }

    echo "branches on origin ($origin_note)"
    while IFS= read -r branch; do
      [ -n "$branch" ] || continue
      [ "$branch" = "develop" ] && { say ok develop "the base"; continue; }
      [ "$branch" = "master" ] && { say ok master "takes develop through a PR"; continue; }
      if hit=$(live_plan_for "$branch"); then
        say ok "$branch" "-> $(basename "${hit%%$'\t'*}") [${hit##*$'\t'}]"
      else
        say "no plan" "$branch" "answers to an issue, or needs a plan's Branch: line (see the board)"
      fi
    done < "$remotes"

    echo ""
    echo "plans naming a branch"
    while IFS= read -r plan; do
      branch=$(plan_branch "$plan"); [ -n "$branch" ] || continue
      st=$(plan_status "$plan")
      case "$st" in Active|Blocked) ;; *) continue ;; esac
      if on_origin "$branch"; then
        say ok "$(basename "$plan")" "$branch [$st]"
      else
        say UNPUSHED "$(basename "$plan")" "$st, names $branch, which origin does not have"
      fi
    done < <(plans)

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
    # `(develop)` once its PR merges into develop, `(master)` once that reaches
    # master.
    while IFS= read -r plan; do
      st=$(plan_status "$plan")
      case "$st" in
        Draft|Active|Blocked|Superseded) ;;
        "Completed (local)"|"Completed (develop)"|"Completed (master)"|"Completed (spike)") ;;
        # Plans completed before 2026-10-06 say only "Completed".
        Completed) ;;
        *) say STALE "$(basename "$plan")" "not a plan status: '$st'" ;;
      esac
    done < <(plans)


    echo ""
    [ "$ok" = 0 ] && echo "wiki and origin agree." || echo "Push what is missing, or update the plan Status: lines above."
    exit "$ok"
