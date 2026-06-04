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
    dist plan

release-build:
    dist build

release-e2e category="smoke":
    cargo build --bin llm-wiki
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- {{category}} --artifact target/debug/llm-wiki

release-e2e-skip-infra category="smoke":
    cargo build --bin llm-wiki
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- {{category}} --artifact target/debug/llm-wiki --skip-infra

release-e2e-linux artifact target_triple="aarch64-unknown-linux-gnu" platform="linux/arm64" image="debian:bookworm-slim":
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- linux --artifact "{{artifact}}" --target-triple "{{target_triple}}" --docker-platform "{{platform}}" --docker-image "{{image}}"

release-e2e-linux-skip-infra artifact target_triple="aarch64-unknown-linux-gnu" platform="linux/arm64" image="debian:bookworm-slim" network="none":
    cargo run --manifest-path tools/release-e2e/Cargo.toml -- linux --artifact "{{artifact}}" --target-triple "{{target_triple}}" --docker-platform "{{platform}}" --docker-image "{{image}}" --docker-network "{{network}}" --skip-infra

release-e2e-linux-build platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_dir="target/release-e2e-linux-aarch64":
    mkdir -p target/release-e2e-docker-home target/release-e2e-cargo-home "{{target_dir}}"
    docker build --platform "{{platform}}" -t "{{image}}" -f infra/release-e2e/linux-builder.Dockerfile infra/release-e2e
    docker run --rm --platform "{{platform}}" --user "$(id -u):$(id -g)" -e HOME=/work/target/release-e2e-docker-home -e CARGO_HOME=/work/target/release-e2e-cargo-home -e CARGO_TARGET_DIR=/work/{{target_dir}} -v "{{justfile_directory()}}:/work" -w /work "{{image}}" cargo build --bin llm-wiki --release
    file "{{target_dir}}/release/llm-wiki"

release-e2e-linux-build-and-test platform="linux/arm64" image="llm-wiki-release-e2e-linux-builder:bookworm" target_dir="target/release-e2e-linux-aarch64" artifact_image="debian:bookworm-slim":
    just release-e2e-linux-build platform="{{platform}}" image="{{image}}" target_dir="{{target_dir}}"
    just release-e2e-linux "{{target_dir}}/release/llm-wiki" platform="{{platform}}" image="{{artifact_image}}"

release-e2e-linux-infra-up stack="e2e":
    cd infra/release-e2e && pulumi stack select "{{stack}}" --create && pulumi up --yes --parallel 1

release-e2e-linux-infra-down stack="e2e":
    cd infra/release-e2e && pulumi stack select "{{stack}}" && pulumi destroy --yes

git-summary:
    git status --short --branch
    git log --oneline --decorate --max-count=14
