#!/usr/bin/env bash
# Schreibt den Review-Stempel für den aktuell gestagten Stand.
# Nur der passende Reviewer-Agent darf das aufrufen (erzwingt .claude/hooks/guard.sh).
#
#   scripts/review-stamp.sh <stempel-id> <PASS|FAIL> ["Grund für Ohne-Testliste"]
#
# Stempel-IDs stehen in scripts/lib.sh. Der Stempel liegt in review_dir <tree>; <tree> ist
# `git write-tree`, also der Fingerabdruck genau des gestagten Stands. Ändert sich danach etwas,
# passt er nicht mehr. Ein FAIL bleibt FAIL, bis sich der gestagte Stand ändert: kein Neu-Würfeln.
set -euo pipefail
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

reviewer=${1:-}; verdict=${2:-}; reason=${3:-}
[ -n "$(reviewer_agent "$reviewer")" ] || { echo "Unbekannte Stempel-ID. Erlaubt: $(reviewer_ids | tr '\n' ' ')" >&2; exit 2; }
case $verdict in PASS|FAIL) ;; *) echo "Urteil muss PASS oder FAIL sein" >&2; exit 2 ;; esac
if [ -n "$reason" ] && [ "$reviewer" != testlist ]; then
  echo "Nur der testlist-Reviewer bestätigt einen Ohne-Testliste-Grund" >&2; exit 2
fi

# Eigene Zuweisung: Nur so bricht set -e ab, wenn write-tree scheitert (z. B. index.lock belegt).
tree=$(git write-tree)
dir=$(review_dir "$tree")
if [ "$(head -n 1 "$dir/$reviewer" 2>/dev/null)" = FAIL ] && [ "$verdict" = PASS ]; then
  echo "Für diesen Stand steht schon FAIL. Erst den Code ändern, dann neu prüfen." >&2; exit 1
fi
mkdir -p "$dir"
{
  echo "$verdict"
  if [ -n "$reason" ]; then echo "Ohne-Testliste: $reason"; fi
} > "$dir/$reviewer"
echo "Stempel: $dir/$reviewer → $verdict"
