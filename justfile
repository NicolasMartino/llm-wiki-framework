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
    ! rg -n 'Successor:|Will Supersede On D8 Completion|<!-- CLAUDE -->|<!-- CODEX -->|<!-- END -->|skills/build.sh|bash renderer|build.sh' wiki assets README.md .github src tests crates Cargo.toml
    ! rg -n 'legacy shell renderer|legacy skill render script|legacy render script|<!-- TAG -->' wiki/specs wiki/decisions wiki/plans wiki/roadmaps wiki/index.md README.md AGENTS.MD CLAUDE.md

verify: fmt test clippy-strict snapshots audit-legacy

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

release-plan:
    cargo dist plan

release-build:
    cargo dist build

git-summary:
    git status --short --branch
    git log --oneline --decorate --max-count=14
