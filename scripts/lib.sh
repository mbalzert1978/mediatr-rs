# Gemeinsames Wissen der Prüfkette, an einer Stelle. Wird per `source` eingebunden von
# scripts/check.sh, scripts/review-stamp.sh, scripts/verify-commit.sh, .githooks/pre-commit, .claude/hooks/guard.sh.

# Reviewer als "<stempel-id>:<agent-name>".
REVIEWERS="solid:solid-reviewer dry:dry-reviewer testlist:testlist-reviewer idiom:rust-idiom-reviewer"

reviewer_ids() { local r; for r in $REVIEWERS; do echo "${r%%:*}"; done; }
reviewer_agent() { # reviewer_agent <id> → agent-name, leer wenn unbekannt
  local r; for r in $REVIEWERS; do if [ "${r%%:*}" = "$1" ]; then echo "${r#*:}"; fi; done
}
reviewer_id_of_agent() { # reviewer_id_of_agent <agent-name> → id, leer wenn kein Reviewer
  local r; for r in $REVIEWERS; do if [ "${r#*:}" = "$1" ]; then echo "${r%%:*}"; fi; done
}

# Stempel und Freigabe eines gestagten Stands liegen in <git-dir>/review/<tree>/.
REVIEW_SUBDIR=review
review_dir() { echo "$(git rev-parse --git-dir)/$REVIEW_SUBDIR/$1"; }

# Wo Crates liegen: in der Wurzel und unter crates/<name>/.
CRATE_RE='(crates/[^/]+/)?'
crate_roots() { local c; echo .; for c in crates/*/; do [ -d "$c" ] && echo "${c%/}"; done; }
# Code und Produktionscode, in jeder Crate.
CODE_PATH_RE="^${CRATE_RE}(src/|tests/|examples/|benches/|build\.rs\$)"
PROD_PATH_RE="^${CRATE_RE}(src/|build\.rs\$)"

# Format der Punkte in TESTLIST.md.
OPEN_ITEM_RE='^- \[ \] .+ → .+$'
DONE_ITEM_RE='^- \[x\] .+ → .+ · `([a-z0-9_]+)`$'
# Testnamen der abgehakten Punkte, gelesen von stdin.
listed_names() { sed -nE "s/$DONE_ITEM_RE/\1/p" | sort; }
