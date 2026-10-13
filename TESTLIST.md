# Testliste

Siehe Skill `canon-tdd`.

## Request/Handler — `send`

Quelle: MediatR `test/MediatR.Tests/SendTests.cs`, `SendVoidInterfaceTests.cs`, `ExceptionTests.cs`.

- [x] Request mit registriertem Handler → `send` liefert die Antwort des Handlers · `send_returns_handler_response`
- [x] Request ohne Antwort (Antworttyp `()`) → `send` führt den Handler aus · `send_runs_handler_for_unit_response`
- [x] Zwei Request-Typen mit je eigenem Handler → jeder Request erreicht nur seinen Handler · `send_reaches_only_handler_of_its_request_type`
- [x] Request ohne registrierten Handler → `send` liefert `Err`, kein Panic · `send_without_handler_returns_err`
- [x] Handler liefert einen Fehler → `send` gibt genau diesen Fehler zurück · `send_returns_handler_error`
- [x] Zweimal `register` für denselben Request-Typ → der zuletzt registrierte Handler antwortet · `register_twice_keeps_last_handler`
- [x] Fehler von `send` → lässt sich mit `?` als `Box<dyn std::error::Error>` weitergeben · `send_error_propagates_as_boxed_error`
- [x] Async-Closure als Handler → `register` nimmt sie an, `send` liefert ihre Antwort · `register_accepts_async_closure`
- [x] Request mit eigenem Fehlertyp ohne registrierten Handler → `send` liefert den aus `MissingHandler` gewandelten Fehler · `send_without_handler_converts_missing_handler_into_request_error`

## Mediator — Fassade und DI

Quelle: MediatR `Mediator(IServiceProvider)` speichert keine Handler; es holt sie bei jedem `Send` über `GetRequiredService` (`src/MediatR/Mediator.cs`, `Wrappers/RequestHandlerWrapper.cs`). Nutzer sehen nur `Mediator::builder()`; Registry und Provider liegen dahinter.

- [ ] Handler bekommt einen Sender per Konstruktor (DI, wie `ISender`) und schickt darüber einen Request → seine Antwort enthält die Antwort des inneren Handlers
- [ ] Handler bekommt einen Sender per Konstruktor, der innere Request scheitert → der Handler gibt den Fehler über seinen eigenen Fehlertyp weiter

## Provider-Crate `mediatr-provider`

Quelle: Lebensdauern von `Microsoft.Extensions.DependencyInjection` (`AddSingleton`, `AddScoped`, `AddTransient`). Der Container registriert und baut Dienste; vom Mediator weiß er nichts.

- [x] Dienst mit `instance` registriert → jede Abfrage beim Provider liefert dasselbe Objekt · `instance_returns_same_object_on_every_get`
- [x] Dienst nicht registriert → die Abfrage beim Provider liefert `None` · `get_without_registration_returns_none`
- [ ] Zweimal `instance` für denselben Typ → die Abfrage liefert den zuletzt registrierten Dienst
- [ ] Dienst mit `transient` registriert → die Bauanleitung läuft bei jeder Abfrage beim Provider neu
- [ ] Dienst mit `singleton` registriert → die Bauanleitung läuft einmal, bei der ersten Abfrage beim Provider
- [ ] Dienst mit `scoped` registriert → die Bauanleitung läuft einmal pro Scope; ein neuer Scope baut neu
