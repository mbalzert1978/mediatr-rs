---
name: canon-tdd
description: Canon-TDD-Ablauf und Testlisten-Regeln für mediatr-rs. Use before writing any test or any code in src/ or tests/, before editing TESTLIST.md, and whenever new behavior is added or a bug is fixed.
---

# Canon TDD in mediatr-rs

Nach Kent Beck, [Canon TDD](https://newsletter.kentbeck.com/p/canon-tdd).
Die **Testliste** gibt die Tests vor, nicht der Code. Wir messen Testlisten-Abdeckung, keine Code-Abdeckung.

## Ablauf

1. **Testliste schreiben.** Trag in `TESTLIST.md` alle erwarteten Varianten des neuen Verhaltens ein: den Grundfall und jedes „was wenn“ (z. B. „was, wenn kein Handler registriert ist“). Ändert sich bestehendes Verhalten, trag auch alles ein, was dabei nicht kaputtgehen darf. Für ein neues MediatR-Konzept lies vorher `docs/mediatr-concepts.md`.
   Fertig, wenn jede Variante, die dir einfällt, als `- [ ]`-Punkt dasteht.
   *Fehler nach Beck:* Implementierungs-Entscheidungen auf die Liste schreiben. Die Liste überspringen.
2. **Genau einen Punkt in einen echten Test verwandeln.** Ein automatisierter Test mit Aufbau, Aufruf, Assertion. Wähl die Reihenfolge bewusst; sie prägt das Ergebnis.
   Lass ihn laufen und sieh ihn **rot**. Kompiliert er nicht, leg nur so viel Signatur an, dass er kompiliert und an der Assertion scheitert.
   Fertig, wenn genau ein neuer Test an seiner Assertion scheitert.
   *Fehler nach Beck:* Test ohne Assertion. Mehrere Punkte auf einmal in Test-Code verwandeln.
3. **Code ändern, bis dieser und alle alten Tests grün sind.** Nur so viel, wie der Test verlangt. Fällt ein neuer Fall auf: als `- [ ]` auf die Liste, nicht umsetzen. Macht ein neuer Test frühere Arbeit ungültig: bewusst entscheiden, weitermachen oder neu anfangen.
   Dann den Punkt abhaken: ``- [x] … · `test_name` ``.
   Fertig, wenn `cargo test` grün ist.
   *Fehler nach Beck:* Assertions löschen oder abschwächen. Den tatsächlichen Wert in die Erwartung kopieren. Hier schon refaktorieren („make it run, then make it right“).
4. **Optional refaktorieren.** Jetzt fallen Implementierungs-Entscheidungen. Zusammenführen nach `docs/rules/dry.md`. Nach jedem Schritt bleibt alles grün.
   *Fehler nach Beck:* Weiter refaktorieren, als diese Sitzung braucht. Zu früh abstrahieren.
5. **Zurück zu Schritt 2, bis kein `- [ ]` mehr im Abschnitt steht.**

Ein Zyklus (Schritt 2–3, mit oder ohne 4) ist ein Commit. Ein reiner Refactor-Commit danach trägt `Ohne-Testliste: Refactor, Verhalten gleich`.

## Testliste

**Ort:** `TESTLIST.md` im Repo-Root. Ein `##`-Abschnitt pro Verhalten. Fertige Abschnitte bleiben stehen; sie sind das Verzeichnis, welcher Test welches Verhalten belegt.

**Format** (prüft `scripts/check.sh`):

```markdown
- [ ] <Situation> → <beobachtbares Ergebnis>
- [x] <Situation> → <beobachtbares Ergebnis> · `test_name`
```

### Regeln

Quelle der Wahrheit für `testlist-reviewer`. Wann ein Verstoß zählt: `docs/rules/review.md`.

| ID | Regel | Erzwungen durch | Grund |
|---|---|---|---|
| T1 | Kein Test und kein Code ohne Testliste. | `check.sh` (Test ↔ Punkt), `commit-msg` (Code ↔ Haken) | Ohne Liste gibt es kein Kriterium für „fertig“. |
| T2 | Ein Punkt nennt Verhalten, das man von außen beobachtet. Er nennt keine internen Typen, Datenstrukturen oder Algorithmen. Namen der öffentlichen API (`send`, `Err`) sind erlaubt. | Reviewer | Beck: Das Innere entscheidet erst Schritt 4. |
| T3 | Jeder Punkt hat die Form `<Situation> → <beobachtbares Ergebnis>`. | `check.sh` (Form), Reviewer (Inhalt) | Ohne beobachtbares Ergebnis gibt es keine Assertion. |
| T4 | Ein Punkt = ein Test. Der Testname gibt den Punkt wieder. | `check.sh` (eins zu eins), Reviewer (Name passt) | Liste und Tests lassen sich abgleichen. |
| T5 | Die Assertion eines Tests prüft genau das Ergebnis seines Punkts. | Reviewer | Sonst belegt der Haken nichts. |
| T6 | Jeder Test hat mindestens eine Assertion (`assert!`, `assert_eq!`, `assert!(matches!(…))`). Ein Test, der nur `?` nutzt, zählt nicht. | Reviewer | Beck: Tests ohne Assertion sind kein Test. |
| T7 | Bestehende Assertions werden nicht gelöscht, abgeschwächt oder an den tatsächlichen Wert angepasst – außer ihr Punkt ändert sich im selben Diff. Ein gelöschter `[x]`-Punkt braucht einen bestätigten `Ohne-Testliste:`-Grund. | Reviewer, `commit-msg` (gelöschter Punkt) | Beck: so tut ein Test nur so, als wäre er grün; und belegtes Verhalten verschwindet nicht still. |
| T8 | Code tut, was der neu abgehakte Punkt verlangt, und nicht mehr. Zusätzliches Verhalten braucht einen eigenen Punkt. | Reviewer | Testlisten-Abdeckung: jedes Verhalten hat einen Punkt. |
| T9 | Ändert ein Commit Produktionscode (`src/`, `build.rs`) ohne neu abgehakten Punkt, steht in der Nachricht `Ohne-Testliste: <Grund>`. „Refactor“ trägt nur, wenn kein Verhalten sich ändert (T7). Andere Gründe nennen, warum es kein beobachtbares Verhalten gibt. | `commit-msg` (Zeile + Stempel), Reviewer (Grund) | Code außerhalb der Liste muss begründet werden. |
| T10 | Neu entdeckte Fälle kommen als `- [ ]` auf die Liste, nicht direkt in den Code. | Reviewer (über T8) | Jeder Zyklus bleibt klein, jeder Test war einmal rot. |
| T11 | Ändert ein Abschnitt bestehendes Verhalten, listet er auch, was dabei nicht kaputtgehen darf. | Reviewer | Beck: Bei einer Verhaltensänderung gehört auch auf die Liste, was nicht kaputtgehen darf. |
| T12 | Ein Commit hakt höchstens einen Punkt neu ab. „Neu“ heißt: Der Testname stand vorher nicht in `TESTLIST.md`. | `reference-transaction` (früh: `commit-msg`) | Beck: ein Test nach dem anderen, nicht alle auf einmal. |

### Tests

- Verhalten der öffentlichen API wird in `tests/` getestet. Jede Datei dort beginnt mit `//!` (erzwingt `missing_docs`) und nennt den Abschnitt in `TESTLIST.md`.
- Ein `#[cfg(test)] mod tests` in `src/` nur, wenn das Verhalten über die öffentliche API nicht erreichbar ist.
- `check.sh` findet Tests über `cargo test --tests -- --list`, also jede Form (`#[test]`, `#[tokio::test(…)]`, Makros). Testnamen sind im ganzen Repo eindeutig.
- Doctests sind Doku-Beispiele. Sie laufen in `cargo test` mit und müssen grün sein, belegen aber keinen Punkt.
- Kein `#[ignore]`: Ein Test, der nie läuft, belegt nichts (prüft `check.sh`).
- Testfunktionen geben `()` zurück und nutzen `.unwrap()`/`.expect()` statt `?` (in `#[test]` erlaubt). Grund: `panic_in_result_fn` verbietet `assert!` in Funktionen mit `Result`.
- Hilfsfunktionen in `tests/` (z. B. ein Test-Handler, der Aufrufe zählt) halten dieselben Lints ein wie Library-Code: `saturating_add` statt `+`, kein `unwrap`.

### Beispiel

```markdown
## Request/Handler — `send`

- [x] Request mit registriertem Handler → `send` liefert die Antwort des Handlers · `send_returns_handler_response`
- [ ] Request ohne Antwort (Antworttyp `()`) → `send` liefert `Ok(())`
- [ ] Zwei Request-Typen mit je eigenem Handler → jeder Request erreicht nur seinen Handler
- [ ] Request ohne registrierten Handler → `send` liefert `Err`, kein Panic
- [ ] Handler liefert einen Fehler → `send` gibt genau diesen Fehler zurück
```

Verstoß gegen T2 (beschreibt Umsetzung, nicht Verhalten):

```markdown
- [ ] HashMap<TypeId, Box<dyn Any>> für die Handler anlegen → Handler gespeichert
```
