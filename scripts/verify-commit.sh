#!/usr/bin/env bash
# Commit-Regeln an einer Stelle. Aufrufer: .githooks/commit-msg (frühe Rückmeldung)
# und .githooks/reference-transaction (harte Sperre für jede Branch-Änderung).
#
#   scripts/verify-commit.sh <tree> <parent|""> <nachrichten-datei>
#
# 1. pre-commit hat genau diesen Stand freigegeben (.git/review/<tree>/approved).
# 2. Höchstens ein Punkt in TESTLIST.md ist neu abgehakt (T12). "Neu" = Testname, der beim Parent fehlte.
# 3. Produktionscode ohne neu abgehakten Punkt und gelöschte [x]-Punkte brauchen
#    "Ohne-Testliste: <Grund>", vom testlist-reviewer bestätigt (T7, T9).
set -euo pipefail
tree=$1 parent=$2 msg=$3
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"
review=$(review_dir "$tree")

fail() { echo "verify-commit: $*" >&2; exit 1; }

[ -f "$review/approved" ] || fail "Stand $tree wurde nicht von pre-commit freigegeben. Leg den Commit normal mit 'git commit' an."

names() { listed_names | uniq; }
before=$( [ -n "$parent" ] && git show "$parent:TESTLIST.md" 2>/dev/null | names || true)
after=$(git show "$tree:TESTLIST.md" 2>/dev/null | names || true)
added=$(comm -13 <(echo "$before") <(echo "$after") | grep . || true)
removed=$(comm -23 <(echo "$before") <(echo "$after") | grep . || true)

if [ "$(grep -c . <<<"$added" || true)" -gt 1 ]; then
  fail "Der Commit hakt mehrere Punkte neu ab: $(tr '\n' ' ' <<<"$added")— ein Commit = ein TDD-Zyklus = höchstens ein neuer Punkt."
fi

if [ -n "$parent" ]; then files=$(git diff --name-only "$parent" "$tree"); else files=$(git ls-tree -r --name-only "$tree"); fi
need=""
if grep -qE '^(src/|build\.rs$)' <<<"$files" && [ -z "$added" ]; then
  need="Der Commit ändert Produktionscode, hakt aber keinen Punkt in TESTLIST.md neu ab."
fi
if [ -n "$removed" ]; then
  need="Der Commit entfernt abgehakte Punkte: $(tr '\n' ' ' <<<"$removed")"
fi
[ -n "$need" ] || exit 0

reason=$(grep -m 1 '^Ohne-Testliste: ' "$msg" || true)
[ -n "$reason" ] || fail "$need Schreib 'Ohne-Testliste: <Grund>' in die Nachricht, bestätigt vom testlist-reviewer."
grep -qxF "$reason" "$review/testlist" 2>/dev/null \
  || fail "'$reason' wurde vom testlist-reviewer nicht bestätigt. Gib ihm genau diesen Grund mit."
