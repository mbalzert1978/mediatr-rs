# DRY-Regeln (Rust-Form)

Quelle der Wahrheit für `dry-reviewer`. Jede Regel hat eine ID. Wann ein Verstoß zählt: `docs/rules/review.md`.
Leitsatz nach Kent Beck: Duplikation ist ein Hinweis, kein Befehl.

- **D1 Gleiches Wissen, nicht gleicher Text.** Zusammengeführt wird nur, was sich aus demselben Grund ändern würde.
  Testfrage: Wenn sich Stelle A ändert, muss sich Stelle B dann aus demselben Grund mitändern? Nur bei Ja ist es gleiches Wissen.
  Verstoß: Eine gemeinsame Funktion bekommt einen `bool`- oder Modus-Parameter, damit sie zwei Aufrufern mit verschiedenen Gründen dient.
  Grund: Falsch geteilter Code zwingt zwei Konzepte, sich gemeinsam zu ändern.
- **D2 Dreier-Regel.** Beim dritten Vorkommen desselben Wissens wird zusammengeführt. Zwei Vorkommen bleiben getrennt.
  Verstoß: Der Diff erzeugt das dritte Vorkommen und führt nicht zusammen (Vorkommen im ganzen Repo zählen, nicht nur im Diff). Oder der Diff führt schon bei zwei Vorkommen zusammen.
  Grund: Erst drei Vorkommen zeigen das Muster sicher.
- **D3 Keine Abstraktion auf Vorrat.** Ein neuer Trait, ein neuer Generic-Parameter oder ein neues Makro braucht mindestens zwei verschiedene Nutzungen im Repo. Ausnahmen: ein `pub trait`, den Nutzer der Library implementieren sollen, und ein Generic-Parameter an einem `pub`-Typ oder einer `pub fn`, in den Nutzer ihre eigenen Typen einsetzen (`docs/rules/solid.md` D1/D2). Ein Generic an einem privaten oder `pub(crate)`-Item mit genau einem eingesetzten Typ bleibt ein Verstoß.
  Funktionen, die nur zur Lesbarkeit ausgelagert sind, fallen nicht unter D3.
  Verstoß: Trait mit genau einer `impl`, Generic mit genau einem eingesetzten Typ, Makro mit einem Aufruf.
  Grund: Eine Abstraktion ohne zweite Nutzung hat die falsche Form, weil nichts sie geprüft hat.
- **D4 Makros nur, wenn Funktion und Generic es nicht können.** Jedes `macro_rules!` hat einen Doc-Kommentar, der nennt, warum eine Funktion oder ein Generic nicht reicht.
  Grund: Makros sind schwer zu lesen und zu debuggen.
- **D5 Zusammenführen ändert kein Verhalten.** Ein Diff, der Duplikate zusammenführt, ändert keine Assertion in bestehenden Tests.
  Grund: Muss eine Assertion angepasst werden, war es kein Refactoring.
- **D6 In Tests geht Lesbarkeit vor DRY.** Jeder Test zeigt Aufbau, Aufruf und Assertion in seinem eigenen Body. Eine gemeinsame Setup-Funktion erst, wenn drei Tests sie nutzen. Ein Test prüft genau einen Testlisten-Punkt; keine Schleife über eine Fall-Tabelle.
  Grund: Ein Test soll ohne Springen durch Dateien verständlich sein und genau einem Punkt der Testliste entsprechen.
