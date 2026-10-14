# Idiomatisches Rust statt C#-Form

Quelle der Wahrheit für `rust-idiom-reviewer`. Lies das, bevor du Typen, Traits oder öffentliche API entwirfst.
Jede Regel hat eine ID. Wann ein Verstoß zählt: `docs/rules/review.md`.

## C#-Muster → Rust-Idiom

Steht ein Muster aus der linken Spalte im Code, ist das ein Verstoß mit der ID der Zeile.

| ID | C#-Muster | Rust-Idiom | Grund |
| --- | --- | --- | --- |
| C1 | Interface (`IRequestHandler<,>`) | Trait (`RequestHandler`) | Traits sind Rusts Mittel für gemeinsames Verhalten. |
| C2 | Generischer Marker `IRequest<TResponse>` | Trait mit assoziiertem Typ `type Response;` | Ein Request hat genau einen Antworttyp; der Compiler prüft das. |
| C3 | Exceptions | `Result<T, E>` mit `?` | Fehler stehen in der Signatur. |
| C4 | `null` | `Option<T>` | Der Compiler erzwingt die Behandlung von „nichts da“. |
| C5 | Reflection / Assembly-Scan / Auflösen ohne Registrierung | Explizite Registrierung im Builder des Mediators; ein DI-Container nur mit expliziter Registrierung, über Generics oder Trait-Objekte | Rust hat keine Laufzeit-Reflection; Registrierung bleibt im Code sichtbar. |
| C6 | Vererbung (`abstract class`) | Komposition + Default-Methoden im Trait | Rust hat keine Klassen-Vererbung. |
| C7 | Eigener `Unit`-Typ | `()` | Gibt es schon. |
| C8 | `Task<T>` | `async fn` / `Future` | Rusts Form von „Ergebnis kommt später“. |
| C9 | `CancellationToken`-Parameter | Future fallen lassen (drop) | Eine Future bricht ab, sobald niemand sie mehr pollt. |
| C10 | Delegate (`RequestHandlerDelegate`) | Closure / `FnOnce` | Closures sind Rusts Funktionswerte. |
| C11 | Überladung (`Send<T>` / `Send(object)`) | Eine generische Funktion | Rust hat keine Überladung. |
| C12 | Casts aus `object` | Statische Typen; `Any`/`TypeId` nur im Registry-Modul | Typfehler fallen beim Kompilieren auf. |

## Regeln

| ID | Regel | Erzwungen durch | Grund |
| --- | --- | --- | --- |
| R1 | Umwandlungen über Standard-Traits: `From`, `TryFrom`, `AsRef`, `FromStr`, `Display`, `Default`, `IntoIterator`. Testfrage: Gibt es einen Std-Trait mit genau dieser Bedeutung? Dann `impl` statt eigener `as_`/`to_`/`into_`/`from_`-Methode. | Clippy `should_implement_trait`, `from_over_into` (nur Std-Namen); Reviewer | Nutzer bekommen `?`, `.into()` und `parse()` gratis. |
| R2 | Getter heißen wie das Feld: `fn name(&self)`, nicht `fn get_name(&self)`. `get_mut`, `get_unchecked` u. ä. mit weiteren Parametern sind erlaubt. | `scripts/check.sh` | Rust-API-Guidelines C-GETTER. |
| R3 | Kein `I`-Präfix bei Traits, kein `_async`-Suffix bei Funktionen. | `scripts/check.sh` | In Rust haben Traits kein Präfix, und `async fn` ist an der Signatur erkennbar. |
| R4 | Jeder `pub`-Typ implementiert `Debug`. Dazu `Clone`, `PartialEq`, `Eq`, `Hash` per `derive`. Testfrage: Implementieren alle Felder den Trait, und besitzt der Typ keine Ressource (Handle, Verbindung, Lock)? Dann muss er ihn ableiten. `Default` nur, wenn es einen natürlichen Leerwert gibt. Testfrage: Würde ein Nutzer ohne Kontext genau diesen Wert als „leer“ erwarten? | `missing_debug_implementations` (nur `Debug`); Reviewer | Rust-API-Guidelines C-COMMON-TRAITS. |
| R5 | Library-Code löst keine Panic aus: kein `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, `process::exit`, kein Index-Panic, kein `assert!` in Funktionen mit `Result`, keine Rechnung, die überlaufen kann (`checked_*`/`saturating_*`/`wrapping_*` statt `+ - *`). Fehlbare Funktionen geben `Result` zurück. | Clippy (`Cargo.toml` `[lints]`) | Ein Panic reißt das Programm des Nutzers ab. |
| R6 | Jedes `pub`-Item hat `///`-Doku; fehlbare Funktionen haben `# Errors`. | `missing_docs`, `clippy::missing_errors_doc` | Eine Library ohne Doku ist nicht benutzbar. |
| R7 | Funktionen haben höchstens 30 Zeilen und 3 Ebenen Verschachtelung. | `clippy.toml` | Was darüber liegt, macht mehr als eine Sache. |
| R8 | Kein Modul heißt `util`, `helper`, `shared`, `misc`, `common` (Ausnahme: `tests/common/`, Rusts Ort für Test-Setup). `lib.rs` enthält nur `mod`, `pub use` und Crate-Doku. | `scripts/check.sh` (Namen); Reviewer (`lib.rs`) | Solche Module sammeln Code ohne gemeinsame Aufgabe. |
| R9 | Lint-Ausnahmen nur als `#[expect(lint, reason = "…")]`. Der Grund nennt, warum der Lint an genau dieser Stelle falsch liegt. Testfrage: Würde der Grund an jeder anderen Stelle genauso passen? Dann ist er zu allgemein. | `allow_attributes`, `allow_attributes_without_reason`; Reviewer (Grund) | Jede Ausnahme ist begründet und meldet sich, wenn sie überflüssig wird. |
| R10 | Ein Item ist `pub` nur, wenn ein Test in `tests/` oder eine andere `pub`-Signatur es braucht. Sonst `pub(crate)` oder privat. | Reviewer | Jedes `pub`-Item ist ein Versprechen, das wir nicht mehr brechen dürfen. |
| R11 | Neue Dependency nur mit Freigabe des Menschen; Stdlib geht vor. | `guard.sh` fragt bei `Cargo.toml` und `cargo add` | MediatR wirbt mit „no dependencies“. |
| R12 | Kein `include!(…)` und kein `#[path = …]`. | `scripts/check.sh` | Code außerhalb der geprüften Ordner liefe an Review und Testliste vorbei. |
| R13 | Module, Typen und Funktionen heißen wie ihr Gegenstück in MediatR, in Rust-Schreibweise: `Wrappers/RequestHandlerWrapper` → `wrappers::RequestHandlerWrapper`, `Pipeline/` → `pipeline`. Ein Rust-Fachbegriff statt des MediatR-Namens (z. B. `erased` statt `wrappers`) nur, wenn eine Zeile aus der Tabelle oben es verlangt. Testfrage: Hat MediatR für dieses Konzept einen Namen? Dann diesen nehmen. | Reviewer | Wer MediatR kennt, findet jedes Konzept unter seinem Namen. |
