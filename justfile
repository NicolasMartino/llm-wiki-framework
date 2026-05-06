set shell := ["bash", "-cu"]

default:
    @just --list

fmt:
    cargo fmt --all --check

fmt-fix:
    cargo fmt --all

test:
    cargo test --workspace

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

clippy-strict:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -D dead_code

snapshots:
    cargo insta test --workspace --check

coverage:
    cargo llvm-cov --workspace --fail-under-lines 80

udeps:
    cargo +nightly udeps --workspace

audit-legacy:
    ! rg -n 'Successor:|Will Supersede On D8 Completion|<!-- CLAUDE -->|<!-- CODEX -->|<!-- END -->|skills/build.sh|bash renderer|build.sh' wiki skills README.md .github tools crates Cargo.toml
    ! rg -n 'legacy shell renderer|legacy skill render script|legacy render script|<!-- TAG -->' wiki/specs wiki/decisions wiki/plans wiki/roadmaps wiki/index.md README.md AGENTS.md CLAUDE.md

verify: fmt test clippy-strict snapshots audit-legacy

verify-full: verify coverage udeps

build-bin:
    cargo build -p llm-wiki-framework

run *args:
    cargo run -p llm-wiki-framework -- {{args}}

build-skills:
    cargo run -p llm-wiki-framework -- build --out .

build-skills-to out:
    cargo run -p llm-wiki-framework -- build --out "{{out}}"

install:
    cargo run -p llm-wiki-framework -- install

install-force:
    cargo run -p llm-wiki-framework -- install --force

uninstall:
    cargo run -p llm-wiki-framework -- uninstall

status:
    cargo run -p llm-wiki-framework -- status

doctor:
    cargo run -p llm-wiki-framework -- doctor

init path name type="web" scale="small" description="One sentence description.":
    cargo run -p llm-wiki-framework -- init "{{path}}" --non-interactive --name "{{name}}" --description "{{description}}" --type "{{type}}" --scale "{{scale}}"

release-plan:
    cargo dist plan

release-build:
    cargo dist build

git-summary:
    git status --short --branch
    git log --oneline --decorate --max-count=14
