# mediatr-rs

## Oberste Regel

Dieses Repo portiert die C#-Library [MediatR](https://github.com/LuckyPennySoftware/MediatR) nach Rust.
**Idiomatisches Rust hat Vorrang vor der C#-Implementierung.** Wir übernehmen Idee und Verhalten von MediatR, nicht die Form der C#-API.
Wenn ein MediatR-Design und ein Rust-Idiom kollidieren, gewinnt Rust.

## Vor dem Schreiben lesen

- Jeder Test, jede Zeile Code, jede Änderung an `TESTLIST.md` → Skill `canon-tdd`.
- Neues MediatR-Konzept → `docs/mediatr-concepts.md`.
- Typen, Traits, öffentliche API entwerfen → `docs/rules/rust-idioms.md` und `docs/rules/solid.md`.
- Code zusammenführen oder abstrahieren → `docs/rules/dry.md`.

## Commit

Ein TDD-Zyklus ist ein Commit. „Code“ heißt hier: `src/`, `tests/`, `examples/`, `benches/`, `build.rs`. „Produktionscode“ heißt: `src/`, `build.rs`. Beides gilt in der Wurzel-Crate und in jeder Crate unter `crates/`; jeder Pfad in den Regeln ist so zu lesen.
Die Git-Hooks in `.githooks/` blockieren den Commit, wenn eins davon fehlt:

1. `scripts/check.sh` ist grün, und alles ist gestagt.
2. Bei Änderungen an Code oder `TESTLIST.md`: PASS-Stempel von `solid-reviewer`, `dry-reviewer`, `testlist-reviewer` und `rust-idiom-reviewer` für genau den gestagten Stand. Starte alle vier parallel. Jede Änderung am gestagten Stand macht die Stempel ungültig.
3. Höchstens ein neu abgehakter Testlisten-Punkt.
4. Produktionscode ohne neu abgehakten Punkt oder ein gelöschter `[x]`-Punkt: Zeile `Ohne-Testliste: <Grund>` in der Nachricht. Gib denselben Grund vorher dem `testlist-reviewer` mit.

Diese Regeln prüft `.githooks/reference-transaction` bei jeder Branch-Änderung über einen git-Befehl, auch bei `--amend`, `--no-verify`, `merge` und `stash`. Folgen:

- `rebase`, `cherry-pick` und `revert` scheitern fast immer, weil ihre Stände nicht freigegeben sind. Nimm `cherry-pick --no-commit` bzw. `revert --no-commit` und dann einen normalen Commit; einen gescheiterten Rebase beendet `git rebase --abort`.
- Merge-Commits sind nicht vorgesehen.
- Der erste Commit enthält `src/lib.rs` ohne Testlisten-Punkt und braucht deshalb `Ohne-Testliste: Projekt-Grundgerüst, kein Verhalten`.

## Grenzen

- Ob ein Test vor dem Code rot war, sieht keine Prüfung und kein Reviewer; die Reviewer sehen nur den gestagten Endstand. Das hält nur der Ablauf im Skill `canon-tdd`.
- `.claude/hooks/guard.sh` schützt Stempel, Hook-Einstellung, `.git/`-Interna und Regel-Dateien. Er prüft Bash-Befehle nur als Text; zusammengesetzte Namen und Symlinks erkennt er nicht. Wer so `.git/refs/` direkt schreibt, umgeht auch den reference-transaction-Hook. Reviewer sollen nur lesen, das erzwingt nur ihr fehlendes Write/Edit. Die Hooks laufen nur lokal; es gibt keine CI, weil das Repo noch kein Remote hat.
- Regeln und Prüfungen (`docs/rules/`, `.githooks/`, `scripts/`, `.claude/`, `Cargo.toml`, `clippy.toml`, diese Datei) ändern sich nur mit Freigabe des Menschen. `guard.sh` fragt bei Edit/Write und bei Bash-Befehlen mit direktem Pfad nach.
- Lint-Ausnahmen nur als `#[expect(clippy::…, reason = "…")]`.
