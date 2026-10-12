# Ablauf für alle Reviewer

Gilt für `solid-reviewer`, `dry-reviewer`, `testlist-reviewer` und `rust-idiom-reviewer`. Jeder Reviewer prüft genau eine Regeldatei; Funde zu anderen Regeldateien lässt er weg.

## Wann ein Verstoß zählt

Ein Verstoß zählt nur mit allen vier Angaben: Regel-ID, `datei:zeile`, wörtliches Zitat, Grund in einem Satz.
Was Clippy oder `scripts/check.sh` schon erzwingen, prüfst du nicht nochmal.

## Ablauf

1. Lies `git diff --cached --stat` und `git diff --cached`. Ist der Diff leer: melde „Nichts gestagt“ und hör auf, ohne Stempel.
2. Lies deine Regeldatei ganz.
3. Lies jede geänderte Datei ganz im gestagten Stand (`git show :<pfad>`), nicht nur die Hunks.
4. Wende jede Regel deiner Regeldatei auf jede geänderte Datei an. Notiere jeden Verstoß mit den vier Angaben oben.
5. Urteil: **PASS** bei null Verstößen, sonst **FAIL**.
6. Stempeln: `scripts/review-stamp.sh <deine-stempel-id> PASS` oder `… FAIL`.
7. Antworte mit dem Bericht.

Fertig bist du, wenn jede Regel deiner Regeldatei gegen jede geänderte Datei geprüft und der Stempel geschrieben ist.

Du liest nur. Dein einziger schreibender Befehl ist `scripts/review-stamp.sh`. Korrekturen schlägst du im Bericht vor; umsetzen tut sie der Haupt-Agent.

## Bericht

```
Urteil: PASS | FAIL
Geprüfte Dateien: <liste>
Verstöße:
- [<ID>] <datei:zeile> — "<zitat>" — <grund> — Vorschlag: <korrektur>
```
