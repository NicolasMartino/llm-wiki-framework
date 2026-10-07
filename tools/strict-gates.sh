#!/usr/bin/env bash
# The strictest gates (wiki/decisions/poman-lives-in-this-workspace.decision.md),
# run over the strict crates. Every gate runs even after one fails; the summary
# counts how many ran, passed, were skipped and failed, and the script exits
# non-zero on any failure or skip. A skip is never a pass: a missing tool, or a
# kind of test with no test in it, counts against the run.
#
# Usage: tools/strict-gates.sh [gate...]   (no argument runs every gate)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# The crates held to the strictest gates. llm-wiki's own modules join them one
# at a time (the ratchet, PM8).
CRATES=(llm-wiki-core poman)
CRATE_DIRS=(crates/llm-wiki-core crates/poman)
# The library lines the coverage and mutation gates leave out: the test files,
# and poman's binary entry point, which only calls the library.
NOT_LIBRARY='(/tests/|/tests\.rs$|crates/poman/src/main\.rs$)'
OUT="$ROOT/target/strict-gates"
NIGHTLY="$(tr -d '[:space:]' < tools/udeps-nightly 2>/dev/null)"

ran=0 passed=0 skipped=0 failed=0
results=()

say() { printf '%s\n' "$*"; }

# record <name> <status> <detail>
record() {
  ran=$((ran + 1))
  case "$2" in
    passed) passed=$((passed + 1)) ;;
    skipped) skipped=$((skipped + 1)) ;;
    *) failed=$((failed + 1)) ;;
  esac
  results+=("$(printf '  %-8s %-28s %s' "$2" "$1" "$3")")
  say "--> $1: $2${3:+ ($3)}"
}

# need <tool> <how to install>: true when the tool is there, else says how
# to get it.
need() {
  if command -v "$1" >/dev/null 2>&1; then return 0; fi
  say "missing tool: $1; install it with: $2"
  return 1
}

need_cargo_tool() {
  if cargo "$1" --version >/dev/null 2>&1; then return 0; fi
  say "missing tool: cargo-$1; install it with: $2"
  return 1
}

# gate <name> <command...>: runs the command, recording passed or failed.
gate() {
  local name="$1"; shift
  say ""
  say "== $name: $*"
  if "$@"; then record "$name" passed ""; else record "$name" failed "see the output above"; fi
}

pin() { python3 -c 'import tomllib; print(tomllib.load(open("rust-toolchain.toml","rb"))["toolchain"]["channel"])'; }

check_toolchain() {
  need python3 "your package manager (python3 3.11 or newer, for tomllib)" || return 1
  need rustup "https://rustup.rs" || return 1
  local want got ok=0 name version edition
  want="$(pin)" || return 1
  got="$(rustc --version | awk '{print $2}')"
  say "pinned toolchain: $want; active rustc: $got"
  [ "$got" = "$want" ] || { say "active rustc $got is not the pin $want"; ok=1; }
  while IFS=$'\t' read -r name version edition; do
    say "crate $name: rust-version $version, edition $edition"
    [ "$version" = "$want" ] || { say "crate $name: rust-version $version is not the pin $want"; ok=1; }
    [ "$edition" = "2024" ] || { say "crate $name: edition $edition is not 2024"; ok=1; }
  done < <(cargo metadata --no-deps --format-version 1 | python3 -c '
import json, sys
strict = set(sys.argv[1:])
for package in json.load(sys.stdin)["packages"]:
    if package["name"] in strict:
        print(package["name"], package.get("rust_version") or "none", package["edition"], sep="\t")
' "${CRATES[@]}")
  if [ -z "$NIGHTLY" ]; then
    say "tools/udeps-nightly names no nightly"; ok=1
  elif ! rustup run "$NIGHTLY" rustc --version >/dev/null 2>&1; then
    say "missing tool: the $NIGHTLY toolchain; install it with: rustup toolchain install $NIGHTLY --profile minimal"; ok=1
  else
    local nightly_version
    nightly_version="$(rustup run "$NIGHTLY" rustc --version | awk '{print $2}' | sed 's/-.*//')"
    say "unused-dependencies nightly: $NIGHTLY (rustc $nightly_version)"
    if [ "$(printf '%s\n%s\n' "$want" "$nightly_version" | sort -V | head -1)" != "$want" ]; then
      say "nightly rustc $nightly_version is older than the pin $want"; ok=1
    fi
  fi
  return "$ok"
}

# The lint table: each strict crate takes the workspace's, which must set
# every strict lint to forbid and each clippy group to deny or forbid; and no
# clippy.toml may lift a lint in test code (the owner, 2026-10-06: "lint all").
check_lint_table() {
  need python3 "your package manager (python3 3.11 or newer, for tomllib)" || return 1
  python3 - "${CRATE_DIRS[@]}" <<'PY'
import pathlib, sys, tomllib

failures = []
root = tomllib.load(open("Cargo.toml", "rb"))
lints = root.get("workspace", {}).get("lints", {})
forbid = {
    "rust": ["unsafe_code", "missing_docs"],
    "rustdoc": [
        "bare_urls", "broken_intra_doc_links", "invalid_codeblock_attributes",
        "invalid_html_tags", "invalid_rust_codeblocks", "missing_crate_level_docs",
        "private_doc_tests", "private_intra_doc_links", "redundant_explicit_links",
        "unescaped_backticks",
    ],
    "clippy": [
        "dbg_macro", "exit", "expect_used", "indexing_slicing", "panic",
        "print_stderr", "print_stdout", "string_slice", "todo", "unimplemented",
        "unreachable", "unwrap_used",
    ],
}

def level(value):
    return value.get("level") if isinstance(value, dict) else value

for table, names in forbid.items():
    for name in names:
        got = level(lints.get(table, {}).get(name))
        if got != "forbid":
            failures.append(f"[workspace.lints.{table}] {name} is {got or 'missing'}, not forbid")
for group in ["pedantic", "nursery", "cargo"]:
    got = level(lints.get("clippy", {}).get(group))
    if got not in ("deny", "forbid"):
        failures.append(f"[workspace.lints.clippy] {group} is {got or 'missing'}, not deny or forbid")

for crate in sys.argv[1:]:
    manifest = tomllib.load(open(f"{crate}/Cargo.toml", "rb"))
    if manifest.get("lints") != {"workspace": True}:
        failures.append(f"{crate}/Cargo.toml: [lints] must be exactly `workspace = true`")

for path in [pathlib.Path("."), *map(pathlib.Path, sys.argv[1:])]:
    for name in ("clippy.toml", ".clippy.toml"):
        config = path / name
        if config.exists():
            for key in tomllib.load(open(config, "rb")):
                if key.startswith("allow-") and key.endswith("-in-tests"):
                    failures.append(f"{config}: {key} lifts a lint in test code")

for failure in failures:
    print(failure)
print(f"lint table: {len(failures)} problem(s)")
sys.exit(1 if failures else 0)
PY
}

# Unit tests sit in their own file (`tests.rs` beside the module, declared with
# `#[cfg(test)] mod tests;`), never inline in the module they test.
check_test_files() {
  need python3 "your package manager" || return 1
  python3 - "${CRATE_DIRS[@]}" <<'PY'
import pathlib, re, sys

failures = []
inline = re.compile(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{")
for crate in sys.argv[1:]:
    for path in sorted(pathlib.Path(crate, "src").rglob("*.rs")):
        text = path.read_text()
        if path.name == "tests.rs":
            continue
        if inline.search(text):
            failures.append(f"{path}: holds an inline test module; move it to a tests.rs of its own")
        elif "#[test]" in text:
            failures.append(f"{path}: holds a #[test] outside a tests.rs")
for failure in failures:
    print(failure)
print(f"test files: {len(failures)} problem(s)")
sys.exit(1 if failures else 0)
PY
}

# test_kind <crate> <kind> <cargo test args...>: one kind of test in one crate.
# A kind that runs no test counts as skipped, never passed.
test_kind() {
  local crate="$1" kind="$2"; shift 2
  local name="tests:$crate:$kind" log count
  say ""
  say "== $name: cargo test -p $crate $*"
  log="$(mktemp)"
  if cargo test -p "$crate" "$@" 2>&1 | tee "$log"; then
    count="$(awk '/^test result:/ {sum += $4} END {print sum + 0}' "$log")"
    if [ "$count" -gt 0 ]; then
      record "$name" passed "$count tests"
    else
      record "$name" skipped "no test of this kind"
    fi
  else
    record "$name" failed "see the output above"
  fi
  rm -f "$log"
}

run_tests() {
  local i crate dir file test_name
  for i in "${!CRATES[@]}"; do
    crate="${CRATES[$i]}" dir="${CRATE_DIRS[$i]}"
    test_kind "$crate" unit --lib
    test_kind "$crate" doc --doc
    if [ -f "$dir/tests/properties.rs" ]; then
      test_kind "$crate" property --test properties
    else
      record "tests:$crate:property" skipped "no $dir/tests/properties.rs"
    fi
    local integration=()
    for file in "$dir"/tests/*.rs; do
      [ -e "$file" ] || continue
      test_name="$(basename "$file" .rs)"
      [ "$test_name" = properties ] || integration+=(--test "$test_name")
    done
    if [ "${#integration[@]}" -gt 0 ]; then
      test_kind "$crate" integration "${integration[@]}"
    else
      record "tests:$crate:integration" skipped "no integration test file in $dir/tests"
    fi
  done
}

package_args() { local crate; for crate in "${CRATES[@]}"; do printf -- '-p\n%s\n' "$crate"; done; }
mapfile -t PACKAGES < <(package_args)

run_fmt() { cargo fmt "${PACKAGES[@]}" --check; }
run_clippy() { cargo clippy "${PACKAGES[@]}" --all-targets --all-features -- -D warnings; }
run_doc() { RUSTDOCFLAGS="-D warnings" cargo doc --no-deps "${PACKAGES[@]}"; }

run_coverage() {
  need_cargo_tool llvm-cov "cargo install cargo-llvm-cov --locked (and the llvm-tools-preview component, which rust-toolchain.toml lists)" || return 1
  cargo llvm-cov "${PACKAGES[@]}" --all-targets --ignore-filename-regex "$NOT_LIBRARY" --fail-under-lines 100 --show-missing-lines
}

run_mutants() {
  need_cargo_tool mutants "cargo install cargo-mutants --locked" || return 1
  mkdir -p "$OUT"
  # Exit codes: 2 a mutant was missed, 3 one timed out; both fail the gate.
  cargo mutants "${PACKAGES[@]}" --exclude 'crates/poman/src/main.rs' --output "$OUT" --no-shuffle
}

run_deny() {
  need_cargo_tool deny "cargo install cargo-deny --locked" || return 1
  local dir ok=0
  # Each strict crate's own graph; llm-wiki's joins with the ratchet (PM8).
  # Every check covers dev-dependencies too: duplicates and licences through
  # the two include-dev keys in deny.toml. `cargo deny list` never shows them.
  # P22's plan records a slip in a dev-dependency rejected by each check.
  for dir in "${CRATE_DIRS[@]}"; do
    say "-- cargo deny over $dir"
    cargo deny --manifest-path "$dir/Cargo.toml" check advisories licenses bans sources || ok=1
  done
  return "$ok"
}

run_udeps() {
  [ -n "$NIGHTLY" ] || { say "tools/udeps-nightly names no nightly"; return 1; }
  rustup run "$NIGHTLY" rustc --version >/dev/null 2>&1 || {
    say "missing tool: the $NIGHTLY toolchain; install it with: rustup toolchain install $NIGHTLY --profile minimal"
    return 1
  }
  if ! cargo "+$NIGHTLY" udeps --version >/dev/null 2>&1; then
    say "missing tool: cargo-udeps; install it with: cargo install cargo-udeps --locked"
    return 1
  fi
  cargo "+$NIGHTLY" udeps "${PACKAGES[@]}" --all-targets
}

ALL=(toolchain lint-table test-files fmt clippy doc tests coverage mutants deny udeps)
selected=("$@")
[ "${#selected[@]}" -gt 0 ] || selected=("${ALL[@]}")

for name in "${selected[@]}"; do
  case "$name" in
    toolchain) gate toolchain check_toolchain ;;
    lint-table) gate lint-table check_lint_table ;;
    test-files) gate test-files check_test_files ;;
    fmt) gate fmt run_fmt ;;
    clippy) gate clippy run_clippy ;;
    doc) gate doc run_doc ;;
    tests) run_tests ;;
    coverage) gate coverage run_coverage ;;
    mutants) gate mutants run_mutants ;;
    deny) gate deny run_deny ;;
    udeps) gate udeps run_udeps ;;
    *) record "$name" failed "no such gate; the gates are: ${ALL[*]}" ;;
  esac
done

say ""
say "strict gates over ${CRATES[*]}"
printf '%s\n' "${results[@]}"
say "ran $ran, passed $passed, skipped $skipped, failed $failed"
[ "$failed" -eq 0 ] && [ "$skipped" -eq 0 ]
