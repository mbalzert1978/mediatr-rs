#!/usr/bin/env bash
# PreToolUse-Wächter. Lokal ein Zaun gegen Versehen, keine Mauer: Bash-Befehle prüft er nur als Text,
# zusammengesetzte Namen (x=re; ${x}view) und Symlinks erkennt er nicht.
# Commits sperrt nicht er, sondern .githooks/reference-transaction. Er schützt nur, was diesen Hook trägt:
# - Git-Hooks bleiben eingeschaltet (core.hooksPath).
# - Review-Stempel (.git/review/) schreibt nur scripts/review-stamp.sh, und jeder Reviewer nur seinen eigenen.
# - Reviewer haben kein Write/Edit.
# - Änderungen an Regeln, Prüfungen, Dependencies und Claude-Settings muss der Mensch freigeben.
# Fehler im Wächter blockieren (exit 2), statt durchzulassen.
set -uo pipefail
trap 'echo "guard.sh: interner Fehler, Aufruf blockiert" >&2; exit 2' ERR
# Byteweise arbeiten: Umlaute in Befehlen (z. B. Commit-Nachrichten) bringen awk/tr sonst zum Absturz.
export LC_ALL=C

command -v jq >/dev/null || { echo "guard.sh: jq fehlt, Aufruf blockiert" >&2; exit 2; }
input=$(cat)
jq -e . >/dev/null <<<"$input" || { echo "guard.sh: ungültiges JSON, Aufruf blockiert" >&2; exit 2; }
tool=$(jq -r '.tool_name // ""' <<<"$input")
agent=$(jq -r '.agent_type // ""' <<<"$input")

decide() { # decide <deny|ask> <grund>
  jq -n --arg d "$1" --arg r "$2" \
    '{hookSpecificOutput: {hookEventName: "PreToolUse", permissionDecision: $d, permissionDecisionReason: $r}}'
  exit 0
}

STAMP_ONLY="Review-Stempel schreibt nur scripts/review-stamp.sh, aufgerufen vom passenden Reviewer-Agenten mit seinem eigenen Namen."
READ_ONLY="Reviewer lesen nur."
HOOKS_ON="Git-Hooks bleiben an. Mach die Prüfungen grün, statt sie zu umgehen."
HUMAN="Das ändert Projekt-Regeln, Prüfungen, Dependencies oder Claude-Settings. Der Mensch muss freigeben."

lower() { tr '[:upper:]' '[:lower:]'; }
# Pfad vereinfachen: "./" und ".." auflösen. Das Dateisystem unterscheidet keine Groß-/Kleinschreibung,
# deshalb wird alles klein verglichen.
norm() {
  local res="" s IFS=/
  set -f
  for s in $1; do case $s in ''|.) ;; ..) res=${res%/*} ;; *) res="$res/$s" ;; esac; done
  set +f
  echo "${res:-/}" | lower
}
root=$(norm "${CLAUDE_PROJECT_DIR:-$(pwd)}")
source "${CLAUDE_PROJECT_DIR:-$(pwd)}/scripts/lib.sh"
own_stamp_id() { reviewer_id_of_agent "$agent"; }
stamps=".git/$REVIEW_SUBDIR"

# Geschützte Pfade, relativ zum Projekt-Root, klein geschrieben.
protected="(\.githooks/|scripts/|\.claude/|\.cargo/|docs/rules/|clippy\.toml|${CRATE_RE}cargo\.toml|claude\.md)"
GIT_DIR_ONLY="Git-Interna (.git/) ändern nur git-Befehle; sonst sieht der reference-transaction-Hook die Änderung nicht."
mutating='(>|\btee\b|\bsed[[:space:]]+-[a-z]*i|\bperl[[:space:]]+-[a-z]*i|\b(mv|rm|cp|ln|chmod|truncate|install)[[:space:]]|\bgit[[:space:]]+(checkout|restore|config)\b)'
# Nennt ein Teilbefehl das Stempel-Skript, gilt das als Aufruf – außer der Teilbefehl liest nur.
reading='^(cat|head|tail|less|more|grep|rg|wc|ls|file|stat|diff|git|sed[[:space:]]+-n)\b'
# Innerhalb von Anführungszeichen trennen | ; & > keine Befehle (z. B. grep -E 'a|b').
neutralize_quotes() {
  awk '{ out = ""; q = ""
    for (i = 1; i <= length($0); i++) { c = substr($0, i, 1)
      if (q == "") { if (c == "\047" || c == "\"") q = c }
      else if (c == q) q = ""
      else if (c ~ /[|;&>]/) c = " "
      out = out c }
    print out }'
}

case $tool in
  Write|Edit|MultiEdit|NotebookEdit)
    [ -z "$(own_stamp_id)" ] || decide deny "$READ_ONLY"
    raw=$(jq -r '.tool_input.file_path // .tool_input.notebook_path // ""' <<<"$input")
    case $raw in /*) path=$(norm "$raw") ;; *) path=$(norm "$root/$raw") ;; esac
    if [[ $path == */.claude/settings*.json || $path == */.cargo/config* ]]; then decide ask "$HUMAN"; fi
    case $path in "$root"/*) rel=${path#"$root"/} ;; *) exit 0 ;; esac
    if [[ $rel == "$stamps"* ]]; then decide deny "$STAMP_ONLY"; fi
    if [[ $rel == .git/* ]]; then decide deny "$GIT_DIR_ONLY"; fi
    if grep -qE "^$protected" <<<"$rel"; then decide ask "$HUMAN"; fi
    ;;
  Bash)
    cmd=$(jq -r '.tool_input.command // ""' <<<"$input")
    lc=$(lower <<<"$cmd")
    stripped=$(sed -E 's/[0-9]*>[[:space:]]*\/dev\/null//g; s/[0-9]*>&[0-9]//g' <<<"$lc" | neutralize_quotes)
    # Ein Teilbefehl pro Zeile, getrennt an && || ; |
    segments=$(awk '{ gsub(/&&|\|\||;|\|/, "\n"); print }' <<<"$stripped" | sed -E 's/^[[:space:]]+//')

    if [[ $lc == *"$stamps"* ]]; then decide deny "$STAMP_ONLY"; fi
    if [[ $lc == *hookspath* ]] && ! grep -qE '^git config (--get )?core\.hookspath$' <<<"$lc"; then decide deny "$HOOKS_ON"; fi
    if grep -qE 'git[[:space:]].*config.*\binclude(if)?\.' <<<"$lc"; then decide deny "$HOOKS_ON Git-Config-Includes könnten core.hooksPath überschreiben."; fi
    if grep -qE '\bcargo[[:space:]]+(add|remove|rm)\b' <<<"$lc"; then decide ask "$HUMAN"; fi

    while read -r seg; do
      [ -n "$seg" ] || continue
      if [[ $seg == *review-stamp.sh* ]] && ! grep -qE "$reading" <<<"$seg"; then
        id=$(sed -nE "s/.*review-stamp\.sh[\"']?[[:space:]]+[\"']?([a-z]+).*/\1/p" <<<"$seg")
        [ -n "$(own_stamp_id)" ] && [ "$id" = "$(own_stamp_id)" ] || decide deny "$STAMP_ONLY"
        continue
      fi
      if grep -qE '^git[[:space:]]+commit\b' <<<"$seg"; then continue; fi
      if grep -qE "(^|[[:space:]\"'/=])\.git/" <<<"$seg" && grep -qE "$mutating" <<<"$seg"; then decide deny "$GIT_DIR_ONLY"; fi
      if grep -qE "(^|[[:space:]\"'/=])($protected|\.claude/settings)" <<<"$seg" && grep -qE "$mutating" <<<"$seg"; then
        decide ask "$HUMAN"
      fi
    done <<<"$segments"
    ;;
esac
exit 0
