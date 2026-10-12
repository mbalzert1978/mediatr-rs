# SOLID-Regeln (Rust-Form)

Quelle der Wahrheit für `solid-reviewer`. Jede Regel hat eine ID. Wann ein Verstoß zählt: `docs/rules/review.md`.
Was Clippy schon erzwingt, steht hier nur als Verweis.

## S — Single Responsibility

- **S1** Jede Datei in `src/` beginnt mit einem `//!`-Kommentar. Er nennt die eine Aufgabe des Moduls in einem Satz ohne „und“.
  Testfrage je Item: Lässt sich dieses Item mit dem `//!`-Satz begründen? Ein Nein ist ein Verstoß, ebenso ein fehlender Satz oder ein Satz mit „und“/„and“.
  Grund: Ein Modul mit zwei Aufgaben ändert sich aus zwei Gründen.
- **S2** Code, der Nachrichten weiterleitet (Mediator, Registry, Pipeline), enthält keine Logik für eine bestimmte Nachricht.
  Verstoß: Weiterleitungs-Code prüft Felder oder Werte einer konkreten Nachricht.
  Grund: Fachlogik gehört in den Handler des Nutzers.

## O — Open/Closed

- **O1** Außerhalb des Registry-Moduls gibt es kein `match`, `if` oder `TypeId`-Vergleich über konkrete Request-, Notification- oder Handler-Typen.
  Verstoß: Ein neuer Nachrichtentyp würde eine Änderung in `src/` erfordern.
  Grund: Nutzer erweitern die Library mit eigenen Typen, ohne ihren Code anzufassen.
- **O2** Erweiterungspunkte sind Traits, die Nutzer implementieren. Ein `enum`, das alle möglichen Handler oder Behaviors aufzählt, ist ein Verstoß.
  Grund: Ein geschlossenes `enum` kann der Nutzer nicht erweitern.

## L — Liskov Substitution

- **L1** Jeder `pub trait` hat in seiner Doku einen Abschnitt `# Vertrag`. Er nennt, was jede Implementierung garantieren muss (z. B. „ruft `next` höchstens einmal auf“).
  Verstoß: `pub trait` ohne `# Vertrag`.
  Grund: Ohne geschriebenen Vertrag kann niemand prüfen, ob eine Implementierung austauschbar ist.
- **L2** Jede `impl` eines Traits – auch in Tests – hält den `# Vertrag` ein. Testfrage je Zusage im `# Vertrag`: Erfüllt diese `impl` sie? Ein Verstoß nennt die verletzte Zusage wörtlich. (`todo!()`, `unimplemented!()`, `panic!()` erzwingt schon Clippy.)
  Grund: Der Mediator behandelt jede Implementierung gleich.
- **L3** Default-Methoden eines Traits halten den `# Vertrag` für jede mögliche Implementierung ein.
  Grund: Eine Default-Methode ist Code, den jeder Implementierer erbt.

## I — Interface Segregation

- **I1** Jede Methode eines Traits ist für jeden Implementierer sinnvoll. Eine `impl`, die eine Methode mit leerem Body, `Ok(())` oder `Default::default()` als Platzhalter füllt, ist ein Verstoß; der Trait wird geteilt.
  Grund: Wer eine Methode nicht braucht, soll nicht von ihr abhängen.
- **I2** Unabhängige Fähigkeiten sind getrennte Traits (`Sender` für send, `Publisher` für publish). Wer beides braucht, nimmt beide als Bound oder einen Supertrait mit Blanket-`impl`.
  Verstoß: Ein Trait vereint Fähigkeiten, die Aufrufer einzeln brauchen.
  Grund: Ein Aufrufer, der nur veröffentlicht, soll nicht von `send` abhängen.

## D — Dependency Inversion

- **D1** Abhängigkeiten kommen über Generic-Parameter oder über Felder, die der Konstruktor bekommt. Kein globaler Zustand: `LazyLock`, `OnceLock`, `thread_local!` sperrt Clippy (`clippy.toml`); hier zählt der Rest, z. B. `static` mit `Mutex`/`Atomic*` für Handler oder Registry.
  Grund: Globaler Zustand macht Tests voneinander abhängig.
- **D2** Generics sind Standard. `dyn Trait` nur, wo verschieden typisierte Werte zusammen gespeichert werden. Jede `dyn`-Stelle hat diesen Grund sichtbar im Code (Sammlung verschiedener Typen).
  Verstoß: `dyn Trait`, wo nur ein Typ ankommt.
  Grund: Generics kosten zur Laufzeit nichts und behalten Typinformationen.
