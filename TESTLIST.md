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

- [x] Handler bekommt einen Sender per Konstruktor (DI, wie `ISender`), holt darüber Daten eines anderen Handlers und verarbeitet sie → seine Antwort enthält die verarbeiteten Daten · `handler_processes_data_fetched_through_injected_sender`
- [x] Handler bekommt einen Sender per Konstruktor, der innere Request scheitert → der Handler gibt den Fehler, in seinen eigenen Fehlertyp verpackt, unverändert weiter · `handler_wraps_failed_inner_request_in_own_error`
- [ ] Handler mit `transient` registriert → jedes `send` baut den Handler neu
- [ ] Handler, der einen Sender per Konstruktor bekommt, wird ohne Mediator getestet und bekommt eine Attrappe statt des Senders → er verarbeitet die Antwort der Attrappe
- [ ] Handler mit Sender per Konstruktor ist als `singleton` registriert, der Mediator wird fallen gelassen → der Handler wird freigegeben (sein `Drop` läuft)

## Provider-Crate `mediatr-provider`

Quelle: Lebensdauern von `Microsoft.Extensions.DependencyInjection` (`AddSingleton`, `AddScoped`, `AddTransient`). Der Container registriert und baut Dienste; vom Mediator weiß er nichts.

- [x] Dienst mit `instance` registriert → jede Abfrage beim Provider liefert dasselbe Objekt · `instance_returns_same_object_on_every_get`
- [x] Dienst nicht registriert → die Abfrage beim Provider liefert `None` · `get_without_registration_returns_none`
- [x] Zweimal `instance` für denselben Typ → die Abfrage liefert den zuletzt registrierten Dienst · `instance_twice_returns_last_registered_service`
- [x] Dienst mit `transient` registriert → die Bauanleitung läuft bei jeder Abfrage beim Provider neu · `transient_runs_recipe_on_every_get`
- [x] Dienst mit `singleton` registriert → die Bauanleitung läuft einmal, bei der ersten Abfrage beim Provider · `singleton_runs_recipe_once_on_first_get`
- [x] Dienst mit `scoped` registriert → die Bauanleitung läuft einmal pro Scope; ein neuer Scope baut neu · `scoped_runs_recipe_once_per_scope`
- [x] Dienst mit `singleton` registriert → ein neuer Scope liefert dasselbe Objekt wie der Provider · `singleton_returns_same_object_in_new_scope`
- [x] Bauanleitung fragt den Provider nach einem anderen registrierten Dienst → der gebaute Dienst enthält diesen Dienst · `recipe_gets_other_service_from_provider`
- [x] Bauanleitung von `singleton` fragt nach einem `scoped`-Dienst, erste Abfrage kommt aus einem Scope → das Singleton enthält den `scoped`-Dienst des Providers, nicht den des Scopes · `singleton_built_from_scope_gets_scoped_service_of_provider`
- [x] Bauanleitung fragt den Provider nach ihrem eigenen Typ → diese Abfrage liefert `None`, kein Panic und keine Endlosschleife · `recipe_asking_for_own_type_gets_none`
- [ ] Zwei Bauanleitungen fragen einander ab (A braucht B, B braucht A) → die innere Abfrage nach A liefert `None`, kein Panic und keine Endlosschleife
- [ ] Bauanleitung von `singleton` fragt nach ihrem eigenen Typ → diese Abfrage liefert `None`, kein Panic und keine Endlosschleife
- [ ] Bauanleitung von `scoped` fragt nach ihrem eigenen Typ → diese Abfrage liefert `None`, kein Panic und keine Endlosschleife
- [ ] Bauanleitung bricht mit einem Panic ab, der Aufrufer fängt ihn → die nächste Abfrage nach demselben Typ baut den Dienst wieder
- [x] Scope geklont → Klon und Original liefern denselben `scoped`-Dienst; die Bauanleitung läuft nur einmal · `cloned_scope_returns_same_scoped_service`
