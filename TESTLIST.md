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
- [x] Fehler von `send`, nachdem der Mediator weggefallen ist → lässt sich mit `?` als `Box<dyn std::error::Error>` weitergeben, sein Text nennt den weggefallenen Mediator · `mediator_dropped_error_propagates_as_boxed_error`
- [x] Async-Closure als Handler → `register` nimmt sie an, `send` liefert ihre Antwort · `register_accepts_async_closure`
- [x] Request mit eigenem Fehlertyp ohne registrierten Handler → `send` liefert den aus `SendError::NoHandler` gewandelten Fehler · `send_without_handler_converts_missing_handler_into_request_error`

## Mediator — Fassade und DI

Quelle: MediatR `Mediator(IServiceProvider)` speichert keine Handler; es holt sie bei jedem `Send` über `GetRequiredService` (`src/MediatR/Mediator.cs`, `Wrappers/RequestHandlerWrapper.cs`). Nutzer sehen nur `Mediator::builder()`; Registry und Provider liegen dahinter.

- [x] Handler bekommt einen Sender per Konstruktor (DI, wie `ISender`), holt darüber Daten eines anderen Handlers und verarbeitet sie → seine Antwort enthält die verarbeiteten Daten · `handler_processes_data_fetched_through_injected_sender`
- [x] Handler bekommt einen Sender per Konstruktor, der innere Request scheitert → der Handler gibt den Fehler, in seinen eigenen Fehlertyp verpackt, unverändert weiter · `handler_wraps_failed_inner_request_in_own_error`
- [x] Handler mit `transient` registriert → jedes `send` baut den Handler neu · `transient_builds_handler_on_every_send`
- [x] Handler, der einen Sender per Konstruktor bekommt, wird ohne Mediator getestet und bekommt eine Attrappe statt des Senders → er verarbeitet die Antwort der Attrappe · `handler_processes_response_of_fake_sender`
- [x] Handler mit `singleton` registriert → nur das erste `send` baut den Handler, jedes weitere nutzt denselben · `singleton_builds_handler_only_on_first_send`
- [x] Handler mit Sender per Konstruktor ist als `singleton` registriert, der Mediator wird fallen gelassen → der Handler wird freigegeben (sein `Drop` läuft) · `singleton_handler_with_sender_is_released_with_mediator`
- [x] Handler mit Sender ist als `singleton` registriert und schickt einen Request, nachdem der Mediator weggefallen ist → `send` liefert `Err(SendError::MediatorDropped)`, kein Panic · `singleton_handler_sending_after_mediator_dropped_gets_mediator_dropped`

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
- [x] Zwei Bauanleitungen fragen einander ab (A braucht B, B braucht A) → die innere Abfrage nach A liefert `None`, kein Panic und keine Endlosschleife · `mutually_dependent_recipes_get_none_for_inner_query`
- [x] Bauanleitung von `singleton` fragt nach ihrem eigenen Typ → diese Abfrage liefert `None`, kein Panic und keine Endlosschleife · `singleton_recipe_asking_for_own_type_gets_none`
- [x] Bauanleitung von `scoped` fragt nach ihrem eigenen Typ → diese Abfrage liefert `None`, kein Panic und keine Endlosschleife · `scoped_recipe_asking_for_own_type_gets_none`
- [x] Bauanleitung fragt zweimal nach ihrem eigenen Typ → beide Abfragen liefern `None`, kein Panic und keine Endlosschleife · `recipe_asking_twice_for_own_type_gets_none_both_times`
- [x] Bauanleitung bricht mit einem Panic ab, der Aufrufer fängt ihn → die nächste Abfrage nach demselben Typ baut den Dienst wieder · `recipe_runs_again_after_caught_panic`
- [x] Bauanleitung von `singleton` bricht mit einem Panic ab, der Aufrufer fängt ihn → die nächste Abfrage baut das Singleton · `singleton_recipe_runs_again_after_caught_panic`
- [x] Bauanleitung von `scoped` bricht mit einem Panic ab, der Aufrufer fängt ihn → die nächste Abfrage im selben Scope baut den Dienst · `scoped_recipe_runs_again_after_caught_panic`
- [x] Bauanleitung fragt nach einem anderen Dienst, dessen Bauanleitung mit einem Panic abbricht, der Aufrufer fängt ihn → die nächste Abfrage baut den Dienst samt dem anderen Dienst · `recipe_runs_again_after_caught_panic_in_dependency`
- [x] Scope geklont → Klon und Original liefern denselben `scoped`-Dienst; die Bauanleitung läuft nur einmal · `cloned_scope_returns_same_scoped_service`
- [x] Schwacher Verweis (`downgrade`) auf einen lebenden Provider → `upgrade` liefert einen Provider, der dasselbe Singleton liefert · `upgraded_weak_provider_returns_same_singleton`
- [x] Bauanleitung von `singleton` merkt sich den Provider schwach, der Provider fällt weg → das Singleton wird freigegeben (sein `Drop` läuft) · `singleton_holding_weak_provider_is_released_with_provider`
- [x] Schwacher Verweis, nachdem der Provider weggefallen ist → `upgrade` liefert `None` · `upgrade_after_provider_dropped_returns_none`
- [x] Schwacher Verweis auf einen Scope, der Scope fällt weg, der Provider lebt → `upgrade` liefert `None` · `upgrade_after_scope_dropped_returns_none`
- [x] Schwacher Verweis auf einen lebenden Scope → `upgrade` liefert einen Provider, der denselben `scoped`-Dienst liefert wie der Scope · `upgraded_weak_scope_returns_same_scoped_service`
- [x] Bauanleitung von `scoped` merkt sich den Scope schwach, der Scope fällt weg, der Provider lebt → der Dienst wird freigegeben (sein `Drop` läuft) · `scoped_holding_weak_scope_is_released_with_scope`
- [x] `scoped`-Dienst, abgefragt beim Provider selbst, hält ein Singleton, das sich den Provider schwach merkt; der Provider fällt weg → beide werden freigegeben (ihr `Drop` läuft) · `scoped_service_and_its_singleton_with_weak_provider_are_released_with_provider`
- [x] Zwei Dienste desselben Typs mit `instance` registriert → `all` liefert beide, in der Reihenfolge der Registrierung · `all_returns_every_instance_in_registration_order`
- [ ] Dienst nicht registriert → `all` liefert eine leere Liste
- [ ] Dienst mit `transient` registriert → `all` baut ihn bei jeder Abfrage neu
- [ ] Dienst mit `singleton` registriert → `all` liefert dasselbe Objekt wie `get`
- [ ] Dienst mit `scoped` registriert → `all` liefert im selben Scope dasselbe Objekt wie `get`; ein neuer Scope baut neu
- [ ] Bauanleitung fragt mit `all` nach ihrem eigenen Typ → die Abfrage liefert eine leere Liste, kein Panic und keine Endlosschleife
- [ ] Zwei Dienste desselben Typs mit `scoped` registriert → `all` liefert im selben Scope zwei verschiedene Objekte, je eins pro Bauanleitung

## Notification — `publish`

Quelle: MediatR `test/MediatR.Tests/PublishTests.cs`, `NotificationHandlerTests.cs`; Standard-Publisher `ForeachAwaitPublisher` (nacheinander, erster Fehler stoppt). Gleichzeitiges Ausführen (`TaskWhenAllPublisher`) bekommt einen eigenen Abschnitt.

- [x] Notification mit einem registrierten Handler → `publish` führt den Handler aus · `publish_runs_registered_handler`
- [ ] Notification mit zwei Handlern → beide laufen, in der Reihenfolge der Registrierung
- [x] Notification ohne registrierten Handler → `publish` liefert `Ok(())`, kein Panic · `publish_without_handler_returns_ok`
- [ ] Zwei Notification-Typen mit je eigenem Handler → jede Notification erreicht nur ihre Handler
- [ ] Erster Handler wartet (`await`), bevor er fertig ist → der zweite Handler startet erst danach
- [ ] Erster von zwei Handlern liefert einen Fehler → `publish` gibt genau diesen Fehler zurück, der zweite Handler läuft nicht
- [ ] Derselbe Handler-Typ zweimal für dieselbe Notification registriert → `publish` führt ihn einmal aus, und zwar den zuletzt registrierten
- [ ] Request-Handler bekommt einen Publisher per Konstruktor und meldet darüber ein Ereignis → die Handler dieser Notification laufen
- [ ] Handler, der einen Publisher per Konstruktor bekommt, wird ohne Mediator getestet und bekommt eine Attrappe → die Attrappe sieht die gemeldete Notification
- [ ] Handler mit Publisher ist als `singleton` registriert und meldet eine Notification, nachdem der Mediator weggefallen ist → `publish` liefert `Err(SendError::MediatorDropped)`, kein Panic
- [ ] Fehler von `publish` → lässt sich mit `?` als `Box<dyn std::error::Error>` weitergeben
