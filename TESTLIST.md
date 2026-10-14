# Testliste

Siehe Skill `canon-tdd`.

## Request/Handler — `send`

Quelle: MediatR `test/MediatR.Tests/SendTests.cs`, `SendVoidInterfaceTests.cs`, `ExceptionTests.cs`, `ServiceFactoryTests.cs`, `GenericTypeConstraintsTests.cs`, `MicrosoftExtensionsDI/DerivingRequestsTests.cs`, `MicrosoftExtensionsDI/TypeResolutionTests.cs`. Doppelte Registrierung: `register` ist eine Registrierung von Hand, darum gewinnt wie bei `GetRequiredService` der letzte Handler; der Assembly-Scan (`TryAddTransient`) behielte den ersten.

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
- [ ] Zweimal `register` für denselben Request-Typ → `send` ruft den zuerst registrierten Handler nicht auf

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

Quelle: Lebensdauern von `Microsoft.Extensions.DependencyInjection` (`AddSingleton`, `AddScoped`, `AddTransient`, `TryAddEnumerable` als `unique_instance`); MediatR `test/MediatR.Tests/MicrosoftExtensionsDI/TypeResolutionTests.cs` (`ShouldNotThrowWithMissingEnumerables`). Der Container registriert und baut Dienste; vom Mediator weiß er nichts.

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
- [x] Dienst nicht registriert → `all` liefert eine leere Liste · `all_without_registration_returns_empty_list`
- [x] Dienst mit `transient` registriert → `all` baut ihn bei jeder Abfrage neu · `transient_all_runs_recipe_on_every_call`
- [x] Dienst mit `singleton` registriert → `all` liefert dasselbe Objekt wie `get` · `singleton_all_returns_same_object_as_get`
- [x] Dienst mit `scoped` registriert → `all` liefert im selben Scope dasselbe Objekt wie `get`; ein neuer Scope baut neu · `scoped_all_returns_same_object_as_get_within_scope`
- [x] Bauanleitung fragt mit `all` nach ihrem eigenen Typ → die Abfrage liefert eine leere Liste, kein Panic und keine Endlosschleife · `recipe_asking_all_for_own_type_gets_empty_list`
- [x] Bauanleitung fragt mit `all` nach ihrem eigenen Typ, die äußere Abfrage kommt über `get` → die innere Abfrage liefert eine leere Liste, kein Panic und keine Endlosschleife · `recipe_asking_all_for_own_type_through_get_gets_empty_list`
- [x] Zwei Bauanleitungen fragen einander mit `all` ab → die innere Abfrage liefert eine leere Liste, kein Panic und keine Endlosschleife · `mutually_dependent_recipes_get_empty_list_from_inner_all`
- [x] Zwei Bauanleitungen desselben Typs, die erste fragt mit `get` nach ihrem eigenen Typ, die äußere Abfrage kommt über `all` → die innere Abfrage liefert `None` · `first_of_two_recipes_asking_for_own_type_through_all_gets_none`
- [x] Zwei Dienste desselben Typs mit `scoped` registriert → `all` liefert im selben Scope zwei verschiedene Objekte, je eins pro Bauanleitung · `two_scoped_registrations_of_same_type_build_one_object_each`
- [x] Zweimal `instance` für denselben Typ, der Provider fällt weg → beide Dienste werden freigegeben (ihr `Drop` läuft) · `both_instances_of_same_type_are_released_with_provider`
- [x] Zweimal `unique_instance` für denselben Typ, beide aus demselben Quelltyp umgewandelt → `all` liefert nur den zuerst registrierten Dienst · `unique_instance_twice_from_same_source_type_all_returns_first`
- [ ] Zweimal `unique_instance` für denselben Typ, beide aus demselben Quelltyp umgewandelt → `get` liefert den zuerst registrierten Dienst
- [ ] Zweimal `unique_instance` für denselben Typ, aus verschiedenen Quelltypen umgewandelt → `all` liefert beide, in der Reihenfolge der Registrierung
- [ ] `unique_instance` aus demselben Quelltyp für zwei verschiedene Typen → jede Abfrage nach einem der beiden Typen liefert ihren Dienst
- [ ] Erst `instance`, dann `unique_instance` für denselben Typ → `all` liefert beide, in der Reihenfolge der Registrierung

## Notification — `publish`

Quelle: MediatR `test/MediatR.Tests/PublishTests.cs`, `ExceptionTests.cs`, `MicrosoftExtensionsDI/Issue1118Tests.cs`, `MicrosoftExtensionsDI/DuplicateAssemblyResolutionTests.cs`; Standard-Publisher `ForeachAwaitPublisher` (nacheinander, erster Fehler stoppt). Doppelte Handler-Typen: `NotificationHandlerWrapper` behält mit `GroupBy(GetType).First()` den zuerst registrierten, anders als `send`. Andere Strategien stehen unter „Notification — Publish-Strategie“.

- [x] Notification mit einem registrierten Handler → `publish` führt den Handler aus · `publish_runs_registered_handler`
- [x] Notification mit zwei Handlern → beide laufen, in der Reihenfolge der Registrierung · `publish_runs_both_handlers_in_registration_order`
- [x] Notification ohne registrierten Handler → `publish` liefert `Ok(())`, kein Panic · `publish_without_handler_returns_ok`
- [ ] Zwei Notification-Typen mit je eigenem Handler → jede Notification erreicht nur ihre Handler
- [ ] Erster Handler wartet (`await`), bevor er fertig ist → der zweite Handler startet erst danach
- [x] Erster von zwei Handlern liefert einen Fehler → `publish` gibt genau diesen Fehler zurück, der zweite Handler läuft nicht · `publish_returns_first_handler_error_and_skips_second_handler`
- [ ] Derselbe Handler-Typ zweimal für dieselbe Notification registriert → `publish` führt ihn einmal aus, und zwar den zuerst registrierten
- [ ] Request-Handler bekommt einen Publisher per Konstruktor und meldet darüber ein Ereignis → die Handler dieser Notification laufen
- [ ] Handler, der einen Publisher per Konstruktor bekommt, wird ohne Mediator getestet und bekommt eine Attrappe → die Attrappe sieht die gemeldete Notification
- [ ] Handler mit Publisher ist als `singleton` registriert und meldet eine Notification, nachdem der Mediator weggefallen ist → `publish` liefert `Err(SendError::MediatorDropped)`, kein Panic
- [ ] Fehler von `publish` → lässt sich mit `?` als `Box<dyn std::error::Error>` weitergeben
- [ ] Notification-Handler mit `transient` registriert → jedes `publish` baut den Handler neu

## Notification — Publish-Strategie

Quelle: MediatR `test/MediatR.Tests/PublishTests.cs`, `NotificationPublisherTests.cs`, `MicrosoftExtensionsDI/NotificationPublisherTests.cs`; `src/MediatR/NotificationPublishers/TaskWhenAllPublisher.cs` (startet alle Handler, meldet den ersten Fehler erst, wenn alle fertig sind).

- [ ] Mediator mit eigener Publish-Strategie, Notification mit zwei Handlern → die eigene Strategie ruft beide Handler auf, beide laufen
- [ ] Strategie „alle gleichzeitig“, zwei Handler, die beide warten (`await`) → der zweite Handler startet, bevor der erste fertig ist
- [ ] Strategie „alle gleichzeitig“, Notification mit drei Handlern, keiner scheitert → `publish` liefert `Ok(())`, jeder Handler lief genau einmal
- [ ] Strategie „alle gleichzeitig“, der erste von zwei Handlern liefert einen Fehler → der zweite Handler läuft trotzdem, `publish` liefert den Fehler des ersten

## Pipeline — Behaviors

Quelle: MediatR `test/MediatR.Tests/PipelineTests.cs`, `MicrosoftExtensionsDI/PipelineTests.cs`, `MicrosoftExtensionsDI/PipeLineMultiCallToConstructorTest.cs`, `SendTests.cs` (Timeout-Tests); `docs/mediatr-concepts.md` (Behavior bricht ab). Das zuerst registrierte Behavior liegt außen. Doppelte Behavior-Typen: `TryAddEnumerable` behält das zuerst registrierte.

- [ ] Zwei Behaviors für einen Request-Typ → das zuerst registrierte läuft außen: außen-vorher, innen-vorher, Handler, innen-nachher, außen-nachher; `send` liefert die Antwort des Handlers
- [ ] Zwei Behaviors für einen Request ohne Antwort (Antworttyp `()`) → sie umschließen den Handler in derselben Reihenfolge, das zuerst registrierte außen
- [ ] Zwei Behaviors für alle Request-Typen → sie umschließen den Handler jedes Requests, das zuerst registrierte außen; `send` liefert die Antwort des Handlers
- [ ] Zwei Behaviors für alle Request-Typen, Request ohne Antwort (Antworttyp `()`) → sie umschließen den Handler in derselben Reihenfolge, das zuerst registrierte außen
- [ ] Zwei Behaviors für alle Request-Typen und eins nur für einen Request-Typ → dieser Request läuft durch alle drei in Registrierungsreihenfolge, ein anderer Request nur durch die zwei allgemeinen; beide bekommen die Antwort ihres Handlers
- [ ] Zwei Behaviors für alle Request-Typen, jedes dreimal abwechselnd registriert → jedes läuft pro `send` genau einmal, in der Reihenfolge seiner ersten Registrierung
- [ ] Zuerst ein Behavior nur für einen Request-Typ ohne Antwort, dann eins für alle Request-Typen registriert → beide laufen in Registrierungsreihenfolge vor dem Handler
- [ ] Handler mit `transient` registriert und von einem Behavior umschlossen → ein `send` baut den Handler genau einmal, das Behavior läuft genau einmal um ihn herum
- [ ] Behavior ruft den nächsten Schritt nicht auf und liefert selbst eine Antwort → `send` liefert diese Antwort, der Handler läuft nicht
- [ ] Behavior setzt eine Frist, der Handler eines Requests mit Antwort braucht länger → `send` liefert `Err` statt einer Antwort, der Handler wird nicht fertig
- [ ] Behavior setzt eine Frist, der Handler eines Requests ohne Antwort (Antworttyp `()`) braucht länger → `send` liefert `Err`, der Handler wird nicht fertig

## Pipeline — Vor- und Nachverarbeiter

Quelle: MediatR `test/MediatR.Tests/Pipeline/RequestPreProcessorTests.cs`, `Pipeline/RequestPostProcessorTests.cs`, `MicrosoftExtensionsDI/PipelineTests.cs`. Vor- und Nachverarbeiter laufen außerhalb aller Behaviors; Nachverarbeiter laufen in Registrierungsreihenfolge, nicht umgekehrt.

- [ ] Request mit registriertem Vorverarbeiter → er läuft vor dem Handler und bekommt den Request; `send` liefert die Antwort des Handlers
- [ ] Request mit registriertem Nachverarbeiter → er läuft nach dem Handler und bekommt Request und Antwort; `send` liefert die Antwort des Handlers
- [ ] Zwei Vorverarbeiter nur für einen Request-Typ und zwei für alle, ebenso vier Nachverarbeiter → alle Vorverarbeiter laufen in Registrierungsreihenfolge vor dem Handler, alle Nachverarbeiter in Registrierungsreihenfolge danach
- [ ] Behaviors, Vor- und Nachverarbeiter teils für alle Request-Typen, teils nur für einen → dieser Request läuft durch Vorverarbeiter, Behaviors (außen nach innen), Handler, Nachverarbeiter; ein anderer Request nur durch die allgemeinen, in derselben Reihenfolge

## Pipeline — Fehler-Handler und Fehler-Aktionen

Quelle: MediatR `test/MediatR.Tests/Pipeline/RequestExceptionHandlerTests.cs`, `Pipeline/RequestExceptionActionTests.cs`, `MicrosoftExtensionsDI/PipelineTests.cs`. Ein Fehler-Handler kann eine Ersatzantwort setzen; eine Fehler-Aktion reagiert nur und läuft, wenn kein Fehler-Handler eine Ersatzantwort gesetzt hat (Standard `ApplyForUnhandledExceptions`). MediatR wählt nach der Fehler-Basisklasse; ohne Vererbung laufen sie in Registrierungsreihenfolge.

- [ ] Handler scheitert, eine Fehler-Aktion ist registriert → die Aktion läuft, `send` liefert trotzdem genau den Fehler des Handlers
- [ ] Handler scheitert, eine Fehler-Aktion ist registriert → sie läuft pro `send` genau einmal
- [ ] Handler scheitert, zwei Fehler-Aktionen sind registriert → beide laufen, `send` liefert den Fehler des Handlers
- [ ] Handler scheitert, zwei Fehler-Handler, der erste setzt eine Ersatzantwort → `send` liefert `Ok` mit dieser Ersatzantwort, der zweite Fehler-Handler läuft nicht
- [ ] Handler scheitert, der Fehler-Handler setzt keine Ersatzantwort → `send` liefert den ursprünglichen Fehler
- [ ] Handler scheitert, der Fehler-Handler scheitert selbst → `send` liefert den Fehler des Fehler-Handlers, nicht den ursprünglichen
- [ ] Handler scheitert, ein Fehler-Handler ohne Ersatzantwort ist registriert → er läuft pro `send` genau einmal
- [ ] Handler scheitert, ein Fehler-Handler setzt eine Ersatzantwort, zusätzlich ist eine Fehler-Aktion registriert → `send` liefert die Ersatzantwort, die Fehler-Aktion läuft nicht
- [ ] Handler scheitert, ein Fehler-Handler ohne Ersatzantwort und eine Fehler-Aktion sind registriert → beide laufen, `send` liefert den Fehler des Handlers
- [ ] Ein Behavior scheitert, bevor der Handler läuft, ein Fehler-Handler ohne Ersatzantwort und eine Fehler-Aktion sind registriert → beide laufen, `send` liefert den Fehler des Behaviors

## Stream-Request — Stream anfordern

Quelle: MediatR `test/MediatR.Tests/CreateStreamTests.cs`; `src/MediatR/Wrappers/StreamRequestHandlerWrapper.cs` (der Stream ist träge: der Handler läuft erst beim Lesen).

- [ ] Stream-Request mit registriertem Stream-Handler → der Stream liefert genau die Elemente des Handlers und endet danach
- [ ] Handler bekommt einen Sender per Konstruktor und fordert darüber einen Stream an → er bekommt die Elemente des Stream-Handlers
- [ ] Stream-Request ohne registrierten Stream-Handler → der Stream liefert einen Fehler, kein Panic

## Stream-Request — Stream-Behaviors

Quelle: MediatR `test/MediatR.Tests/StreamPipelineTests.cs`, `Pipeline/Streams/StreamPipelineBehaviorTests.cs`, `MicrosoftExtensionsDI/StreamPipelineTests.cs`. Das „nachher“ eines Stream-Behaviors läuft erst nach dem letzten Element.

- [ ] Zwei Stream-Behaviors für einen Stream-Request-Typ → die Elemente des Handlers kommen unverändert an; Reihenfolge: erstes vorher, zweites vorher, Handler, zweites nachher, erstes nachher
- [ ] Zwei Stream-Behaviors für alle Stream-Request-Typen → sie umschließen den Handler, das zuerst registrierte außen; die Elemente kommen unverändert an
- [ ] Zwei Stream-Behaviors für alle Stream-Request-Typen und ein drittes nur für einen → bei diesem Typ laufen alle drei in Registrierungsreihenfolge um den Handler, bei einem anderen Stream-Request-Typ nur die zwei allgemeinen
- [ ] Handler liefert drei Elemente, ein Stream-Behavior fügt vorne und hinten je eins an → der Stream liefert genau fünf Elemente in dieser Reihenfolge
