#!/usr/bin/env bash
# Pflicht-Prüfungen. Läuft im pre-commit-Hook; vor jedem Commit muss alles PASS sein.
# Prüft, was eine Maschine sicher prüfen kann. Urteile (SOLID, DRY, Idiome, Testlisten-Inhalt)
# machen die Reviewer-Agenten in .claude/agents/.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
source scripts/lib.sh

# Von außen lassen sich Lints abschalten (RUSTFLAGS="--cap-lints allow", eigener rustc-Wrapper,
# anderes clippy.toml, rustflags in einer Cargo-Config). Leere RUSTFLAGS überschreiben jede Config.
export RUSTFLAGS="" RUSTDOCFLAGS="" CARGO_ENCODED_RUSTFLAGS="" CARGO_BUILD_RUSTFLAGS=""
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_BUILD_RUSTC_WRAPPER CLIPPY_CONF_DIR

failed=0
gate() { # gate <name> <befehl...>
  local name=$1; shift
  local out
  if out=$("$@" 2>&1); then
    echo "PASS  $name"
  else
    echo "FAIL  $name"
    echo "$out" | sed 's/^/      /'
    failed=1
  fi
}

# Treffer = Verstoß: grep findet etwas → Gate rot.
none() { ! "$@"; }

code_paths() { # in jeder Crate
  local c p
  for c in $(crate_roots); do for p in src tests examples benches build.rs; do
    [ -e "$c/$p" ] && p="$c/$p" && echo "${p#./}"
  done; done
}

# Jeder Test, den cargo kennt (auch #[tokio::test(...)], Makro-Tests, Einzeiler), als letzter Pfad-Teil.
# Doctests zählen nicht: Sie sind Doku-Beispiele und laufen in "cargo test" trotzdem mit.
test_names() {
  cargo test --workspace --quiet --tests -- --list --format terse 2>/dev/null \
    | sed -nE 's/^(.*::)?([A-Za-z0-9_]+): test$/\2/p' | sort
}

testlist_format() {
  [ -f TESTLIST.md ] || return 0
  local bad
  bad=$(grep -nE '^[[:space:]]*[-*] \[' TESTLIST.md | while IFS= read -r line; do
    grep -qE "$OPEN_ITEM_RE|$DONE_ITEM_RE" <<<"${line#*:}" || echo "$line"
  done)
  [ -z "$bad" ] || { echo "$bad"; echo 'Format: "- [ ] Situation → Ergebnis" oder "- [x] Situation → Ergebnis · `test_name`"'; return 1; }
}

testlist_checks() {
  local tests listed dup unlisted missing
  tests=$(test_names)
  listed=$( [ -f TESTLIST.md ] && listed_names < TESTLIST.md )
  dup=$( { uniq -d <<<"$tests" | sed 's/^/Testname doppelt im Code: /'; uniq -d <<<"$listed" | sed 's/^/Testname doppelt in TESTLIST.md: /'; } | grep -v ': $')
  unlisted=$(comm -23 <(uniq <<<"$tests") <(uniq <<<"$listed") | grep .)
  missing=$(comm -13 <(uniq <<<"$tests") <(uniq <<<"$listed") | grep .)
  [ -z "$dup" ] || echo "$dup"
  [ -z "$unlisted" ] || sed 's/^/Test ohne [x]-Punkt in TESTLIST.md: /' <<<"$unlisted"
  [ -z "$missing" ] || sed 's/^/[x]-Punkt ohne Test: /' <<<"$missing"
  [ -z "$dup$unlisted$missing" ]
}

# Ein ignorierter Test läuft nie und belegt deshalb keinen Punkt.
ignored_tests() {
  cargo test --workspace --quiet --tests -- --list --format terse --ignored 2>/dev/null | grep ': test$'
}

# Module ohne gemeinsame Aufgabe: als Datei, Ordner oder inline (`mod util {`). tests/common/ ist Rusts Ort für Test-Setup.
bad_names='util utils helper helpers shared misc common'
forbidden_modules() {
  local n dirs found
  dirs=$(code_paths | grep -E '(^|/)(src|tests)$')  # nur vorhandene: find bricht bei fehlendem Ordner ganz ab
  found=$( { for n in $bad_names; do find $dirs \( -iname "$n" -o -iname "$n.rs" \); done | grep -vxE "${CRATE_RE}tests/common"
    grep -rsnE "^[[:space:]]*(pub(\([a-z]+\))?[[:space:]]+)?mod[[:space:]]+(${bad_names// /|})\b" $(grep -E '(^|/)src$' <<<"$dirs"); } || true)
  [ -n "$found" ] && echo "$found"
}

gate "cargo fmt --check"                 cargo fmt --all --check
gate "cargo clippy --all-targets"        cargo clippy --workspace --all-targets --quiet -- -D warnings
gate "cargo test"                        cargo test --workspace --quiet
gate "kein get_-Getter"                  none grep -rsnE '\bfn get_[a-z0-9_]*(<[^>]*>)?\(&self\)' $(code_paths)
gate "kein I-Präfix bei Traits"          none grep -rsnE '\btrait I[A-Z]' $(code_paths)
gate "kein _async-Suffix"                none grep -rsnE '\bfn [a-z0-9_]+_async\b' $(code_paths)
gate "kein include!/#[path]"             none grep -rsnE '#\[path|\binclude!\(' $(code_paths)
gate "keine util/helper/shared/misc-Module" none forbidden_modules
gate "TESTLIST.md Format"                testlist_format
gate "Test ↔ [x]-Punkt eins zu eins"     testlist_checks
gate "kein #[ignore]-Test"               none ignored_tests

exit "$failed"
