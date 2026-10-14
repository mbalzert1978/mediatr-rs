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
- [x] Zweimal `register` für denselben Request-Typ → `send` ruft den zuerst registrierten Handler nicht auf · `register_twice_does_not_call_first_handler`

## Mediator — Fassade und DI

Quelle: MediatR `Mediator(IServiceProvider)` speichert keine Handler; es holt sie bei jedem `Send` über `GetRequiredService` (`src/MediatR/Mediator.cs`, `Wrappers/RequestHandlerWrapper.cs`). Nutzer sehen nur `Mediator::builder()`; Registry und Provider liegen dahinter.

- [x] Handler bekommt einen Sender per Konstruktor (DI, wie `ISender`), holt darüber Daten eines anderen Handlers und verarbeitet sie → seine Antwort enthält die verarbeiteten Daten · `handler_processes_data_fetched_through_injected_sender`
- [x] Handler bekommt einen Sender per Konstruktor, der innere Request scheitert → der Handler gibt den Fehler, in seinen eigenen Fehlertyp verpackt, unverändert weiter · `handler_wraps_failed_inner_request_in_own_error`
- [x] Handler mit `Handler::transient` registriert → jedes `send` baut den Handler neu · `transient_builds_handler_on_every_send`
- [x] Handler, der einen Sender per Konstruktor bekommt, wird ohne Mediator getestet und bekommt eine Attrappe statt des Senders → er verarbeitet die Antwort der Attrappe · `handler_processes_response_of_fake_sender`
- [x] Handler mit `Handler::singleton` registriert → nur das erste `send` baut den Handler, jedes weitere nutzt denselben · `singleton_builds_handler_only_on_first_send`
- [x] Handler mit Sender per Konstruktor ist als `Handler::singleton` registriert, der Mediator wird fallen gelassen → der Handler wird freigegeben (sein `Drop` läuft) · `singleton_handler_with_sender_is_released_with_mediator`
- [x] Handler mit Sender ist als `Handler::singleton` registriert und schickt einen Request, nachdem der Mediator weggefallen ist → `send` liefert `Err(SendError::MediatorDropped)`, kein Panic · `singleton_handler_sending_after_mediator_dropped_gets_mediator_dropped`
- [x] `Handler` für einen Handler-Typ ohne `Debug` mit `{:?}` formatiert → die Ausgabe nennt den Handler-Typ · `handler_debug_names_handler_type_without_debug`

## Provider-Crate `mediatr-provider`

Quelle: Lebensdauern von `Microsoft.Extensions.DependencyInjection` (`AddSingleton`, `AddScoped`, `AddTransient`, `TryAddEnumerable` als `unique_by`); MediatR `test/MediatR.Tests/MicrosoftExtensionsDI/TypeResolutionTests.cs` (`ShouldNotThrowWithMissingEnumerables`). Der Container registriert und baut Dienste; vom Mediator weiß er nichts.

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
- [x] Zweimal `register` für denselben Typ mit `unique_by` und demselben Schlüsseltyp → `all` liefert nur den zuerst registrierten Dienst · `unique_instance_twice_from_same_source_type_all_returns_first`
- [x] Zweimal `register` für denselben Typ mit `unique_by` und demselben Schlüsseltyp → `get` liefert den zuerst registrierten Dienst · `unique_instance_twice_from_same_source_type_get_returns_first`
- [x] Zweimal `register` für denselben Typ mit `unique_by` und verschiedenen Schlüsseltypen → `all` liefert beide, in der Reihenfolge der Registrierung · `unique_instance_from_different_source_types_all_returns_both_in_order`
- [x] `register` mit `unique_by` und demselben Schlüsseltyp für zwei verschiedene Typen → `get` nach jedem der beiden Typen liefert ihren Dienst · `unique_instance_from_same_source_type_for_two_service_types_registers_both`
- [x] `register` mit `unique_by` und demselben Schlüsseltyp für zwei verschiedene Typen → `all` nach jedem der beiden Typen liefert genau ihren Dienst · `unique_instance_from_same_source_type_for_two_service_types_all_returns_each_own`
- [x] Erst `register` ohne, dann mit `unique_by` für denselben Typ → `all` liefert beide, in der Reihenfolge der Registrierung · `instance_then_unique_instance_of_same_type_all_returns_both_in_order`
- [x] `Registration` mit `map` umgewandelt → `get` liefert den umgewandelten Dienst · `mapped_registration_get_returns_converted_service`
- [x] Zweimal `register` für denselben Typ, beide erst mit `unique_by` und demselben Schlüsseltyp, dann mit `map` umgewandelt → `all` liefert nur den zuerst registrierten Dienst · `unique_then_mapped_registration_twice_all_returns_first`
- [x] `Registration::singleton` mit `map` umgewandelt, zweimal `get` → `f` bekommt beide Male dasselbe Objekt · `mapped_singleton_passes_same_object_to_f_on_every_get`
- [x] Zweimal `register` mit `Registration::transient` für denselben Typ, beide mit `unique_by` und demselben Schlüsseltyp → `all` baut nur den zuerst registrierten Dienst · `unique_transient_twice_from_same_source_type_all_builds_only_first`
- [x] `Registration` mit `{:?}` formatiert → die Ausgabe nennt den Diensttyp · `registration_debug_names_service_type`

## Notification — `publish`

Quelle: MediatR `test/MediatR.Tests/PublishTests.cs`, `ExceptionTests.cs`, `MicrosoftExtensionsDI/Issue1118Tests.cs`, `MicrosoftExtensionsDI/DuplicateAssemblyResolutionTests.cs`; Standard-Publisher `ForeachAwaitPublisher` (nacheinander, erster Fehler stoppt). Doppelte Handler-Typen: `NotificationHandlerWrapper` behält mit `GroupBy(GetType).First()` den zuerst registrierten, anders als `send`. Andere Strategien stehen unter „Notification — Publish-Strategie“.

- [x] Notification mit einem registrierten Handler → `publish` führt den Handler aus · `publish_runs_registered_handler`
- [x] Notification mit zwei Handlern → beide laufen, in der Reihenfolge der Registrierung · `publish_runs_both_handlers_in_registration_order`
- [x] Notification ohne registrierten Handler → `publish` liefert `Ok(())`, kein Panic · `publish_without_handler_returns_ok`
- [x] Zwei Notification-Typen mit je eigenem Handler → jede Notification erreicht nur ihre Handler · `publish_reaches_only_handlers_of_its_notification_type`
- [x] Erster Handler wartet (`await`), bevor er fertig ist → der zweite Handler startet erst danach · `publish_starts_second_handler_after_first_handler_finished_waiting`
- [x] Erster von zwei Handlern liefert einen Fehler → `publish` gibt genau diesen Fehler zurück, der zweite Handler läuft nicht · `publish_returns_first_handler_error_and_skips_second_handler`
- [x] Derselbe Handler-Typ zweimal für dieselbe Notification registriert → `publish` führt ihn einmal aus, und zwar den zuerst registrierten · `publish_runs_handler_type_registered_twice_once_with_first_registration`
- [x] Request-Handler bekommt einen Publisher per Konstruktor und meldet darüber ein Ereignis → die Handler dieser Notification laufen · `request_handler_publishes_through_injected_publisher`
- [x] Handler, der einen Publisher per Konstruktor bekommt, wird ohne Mediator getestet und bekommt eine Attrappe → die Attrappe sieht die gemeldete Notification · `handler_publishes_to_fake_publisher`
- [x] Handler mit Publisher ist als `Handler::singleton` registriert und meldet eine Notification, nachdem der Mediator weggefallen ist → `publish` liefert `Err(SendError::MediatorDropped)`, kein Panic · `singleton_handler_publishing_after_mediator_dropped_gets_mediator_dropped`
- [x] Handler mit Publisher ist als `Handler::singleton` registriert und meldet eine Notification, solange der Mediator lebt → die Handler dieser Notification laufen · `singleton_handler_publishing_while_mediator_alive_runs_handlers`
- [x] Fehler von `publish` → lässt sich mit `?` als `Box<dyn std::error::Error>` weitergeben · `publish_error_propagates_as_boxed_error`
- [x] Notification-Handler mit `subscribe` und `Handler::transient` registriert → jedes `publish` baut den Handler neu · `subscribe_transient_builds_handler_on_every_publish`
- [x] Notification-Handler mit `subscribe` und `Handler::transient` meldet über den Mediator, den er bekommt, eine andere Notification → deren Handler laufen · `subscribe_transient_handler_publishes_through_received_mediator`
- [x] Derselbe Handler-Typ einmal mit `Handler::instance`, einmal mit `Handler::transient` für dieselbe Notification registriert → `publish` führt ihn einmal aus, und zwar den zuerst registrierten · `publish_runs_handler_type_subscribed_and_subscribed_transient_once_with_first_registration`
- [x] Derselbe Handler-Typ zweimal mit `subscribe` und `Handler::transient` für dieselbe Notification registriert → `publish` führt ihn einmal aus, und zwar den zuerst registrierten · `publish_runs_handler_type_subscribed_transient_twice_once_with_first_registration`
- [x] Derselbe Handler-Typ erst mit `Handler::transient`, dann mit `Handler::instance` für dieselbe Notification registriert → `publish` baut und führt nur den mit `Handler::transient` registrierten aus · `publish_runs_handler_type_subscribed_transient_then_instance_once_with_transient`
- [x] Zwei verschiedene Handler-Typen mit `subscribe` und `Handler::transient` für dieselbe Notification registriert → `publish` führt beide aus, in der Reihenfolge der Registrierung · `publish_runs_two_handler_types_subscribed_transient_in_registration_order`
- [x] Handler von Notification A meldet über den Mediator, den er bekommt, Notification B, deren Handler über seinen Mediator einen Request schickt → alle Handler laufen, `publish` liefert `Ok(())`, kein Panic · `nested_publish_and_send_through_received_mediators_run_all_handlers_without_panic`

## Notification — Publish-Strategie

Quelle: MediatR `test/MediatR.Tests/PublishTests.cs`, `NotificationPublisherTests.cs`, `MicrosoftExtensionsDI/NotificationPublisherTests.cs`; `src/MediatR/NotificationPublishers/TaskWhenAllPublisher.cs` (startet alle Handler, meldet den ersten Fehler erst, wenn alle fertig sind). Default bleibt „nacheinander“ wie `ForeachAwaitPublisher`: Laufen Handler verschränkt, löst ein `RefCell`-Borrow über ein `await` eine Panic aus; „alle gleichzeitig“ gibt es deshalb nur bewusst gewählt.

- [x] Mediator mit eigener Publish-Strategie, die Handler in umgekehrter Reihenfolge aufruft, Notification mit zwei Handlern → beide laufen, in der Reihenfolge der eigenen Strategie · `custom_strategy_runs_both_handlers_in_its_own_order`
- [x] Mediator mit eigener Publish-Strategie, Request mit registriertem Handler → `send` liefert die Antwort des Handlers · `custom_strategy_send_returns_handler_response`
- [x] Mediator mit eigener Publish-Strategie, Handler mit `Handler::singleton` schickt über den `WeakMediator`, den er bekommt, einen Request → `send` liefert die Antwort von dessen Handler · `custom_strategy_singleton_handler_sends_through_received_weak_mediator`
- [x] Mediator und `WeakMediator` mit eigener Publish-Strategie, die kein `Clone` hat → lassen sich klonen, und der Klon erreicht dieselben Handler · `custom_strategy_without_clone_mediator_and_weak_mediator_clone_reaches_same_handlers`
- [x] Handler mit `Handler::singleton` in einem Mediator mit eigener Publish-Strategie klont den `WeakMediator`, den er bekommt, und meldet über Original und Klon je eine Notification → beide erreichen dieselben Handler · `custom_strategy_singleton_handler_publishes_through_weak_mediator_and_its_clone`
- [x] Mediator, `WeakMediator` und `MediatorBuilder` mit eigener Publish-Strategie, die kein `Debug` hat, mit `{:?}` formatiert → die Ausgabe nennt die Strategie · `custom_strategy_without_debug_mediator_weak_mediator_and_builder_debug_names_strategy`
- [x] `MediatorBuilder` mit eigener Publish-Strategie, die kein `Clone` hat → lässt sich klonen, und beide bauen Mediatoren, die dieselben Handler erreichen · `custom_strategy_without_clone_builder_clone_builds_mediators_reaching_same_handlers`
- [x] Mediator mit eigener Publish-Strategie, Handler mit `Handler::singleton` meldet über den `WeakMediator`, nachdem der Mediator weggefallen ist → `publish` liefert `Err(SendError::MediatorDropped)`, kein Panic · `custom_strategy_singleton_handler_publishing_after_mediator_dropped_gets_mediator_dropped`
- [x] Mediator mit eigener Publish-Strategie, Handler mit `Handler::transient` meldet über den Mediator, den er bekommt, eine andere Notification → auch deren Handler laufen in der Reihenfolge der eigenen Strategie · `custom_strategy_transient_handler_publishes_through_received_mediator_in_its_order`
- [x] Mediator mit eigener Publish-Strategie, Handler mit `Handler::singleton` meldet über den `WeakMediator`, den er bekommt, eine andere Notification → auch deren Handler laufen in der Reihenfolge der eigenen Strategie · `custom_strategy_singleton_handler_publishes_through_received_weak_mediator_in_its_order`
- [x] Strategie „alle gleichzeitig“, zwei Handler, die beide warten (`await`) → der zweite Handler startet, bevor der erste fertig ist · `concurrent_strategy_starts_second_waiting_handler_before_first_finished`
- [x] Strategie „alle gleichzeitig“, zwei Handler, die gleich lange warten → sie werden in der Reihenfolge der Registrierung fertig · `concurrent_strategy_equally_waiting_handlers_finish_in_registration_order`
- [x] Strategie „alle gleichzeitig“, Notification mit drei Handlern, keiner scheitert → `publish` liefert `Ok(())`, jeder Handler lief genau einmal · `concurrent_strategy_runs_each_of_three_handlers_once_and_returns_ok`
- [x] Strategie „alle gleichzeitig“, der erste von zwei Handlern liefert einen Fehler → der zweite Handler läuft trotzdem, `publish` liefert den Fehler des ersten · `concurrent_strategy_runs_second_handler_after_first_fails_and_returns_first_error`
- [x] Strategie „alle gleichzeitig“, beide von zwei Handlern liefern einen Fehler → `publish` liefert den Fehler des zuerst registrierten · `concurrent_strategy_both_handlers_fail_returns_error_of_first_registered`
- [x] Strategie „alle gleichzeitig“, beide von zwei Handlern liefern einen Fehler, der zuerst registrierte erst nach einem `await` → `publish` liefert trotzdem den Fehler des zuerst registrierten · `concurrent_strategy_both_fail_first_after_await_returns_error_of_first_registered`
- [x] Strategie „alle gleichzeitig“, nur der zweite von zwei Handlern liefert einen Fehler → der erste lief, `publish` liefert den Fehler des zweiten · `concurrent_strategy_only_second_handler_fails_runs_first_and_returns_second_error`

## Pipeline — Behaviors

Quelle: MediatR `test/MediatR.Tests/PipelineTests.cs`, `MicrosoftExtensionsDI/PipelineTests.cs`, `MicrosoftExtensionsDI/PipeLineMultiCallToConstructorTest.cs`, `SendTests.cs` (Timeout-Tests); `docs/mediatr-concepts.md` (Behavior bricht ab). Wie bei MediatR gibt es Behaviors für alle Request-Typen und Behaviors nur für einen; man registriert beide Arten in freier Reihenfolge vor dem ersten Handler, das zuerst registrierte liegt außen. Ein mehrfach registriertes Behavior läuft wie bei `TryAddEnumerable` nur einmal, an der Stelle seiner ersten Registrierung. Anders als MediatR überspringt `send` kein Behavior, das mehr vom Request verlangt; es kompiliert dann nicht (Entscheidung Markus).

- [x] Zwei Behaviors, ein Request → das zuerst registrierte läuft außen: außen-vorher, innen-vorher, Handler, innen-nachher, außen-nachher; `send` liefert die Antwort des Handlers · `two_behaviors_wrap_handler_first_registered_outermost`
- [x] Behavior klont den nächsten Schritt und ruft ihn nach einem Fehler des Handlers noch einmal auf → der Handler läuft zweimal, `send` liefert die Antwort des zweiten Laufs · `behavior_retries_cloned_next_after_handler_error_and_returns_second_response`
- [x] Zwei Behaviors, Request ohne Antwort (Antworttyp `()`) → sie umschließen den Handler in derselben Reihenfolge, das zuerst registrierte außen · `two_behaviors_wrap_unit_response_handler_first_registered_outermost`
- [x] Zwei Behaviors, Requests zweier Typen mit je eigenem Handler → jeder Request läuft durch beide Behaviors, das zuerst registrierte außen; `send` liefert jeweils die Antwort seines Handlers · `two_behaviors_wrap_handlers_of_every_request_type_first_registered_outermost`
- [x] Ein Behavior für alle Request-Typen, eins nur für einen, noch eins für alle, in dieser Reihenfolge registriert → ein Request dieses Typs läuft durch alle drei in Registrierungsreihenfolge, das erste außen; ein Request eines anderen Typs nur durch die zwei allgemeinen; beide bekommen die Antwort ihres Handlers · `behavior_for_one_request_type_between_two_for_all_wraps_only_its_requests_in_registration_order`
- [ ] Zuerst ein Behavior nur für einen Request-Typ ohne Antwort (Antworttyp `()`), dann eins für alle registriert → beide laufen in Registrierungsreihenfolge um den Handler, das erste außen; `send` liefert `Ok(())`
- [ ] Zwei Behaviors, jedes dreimal abwechselnd registriert → jedes läuft pro `send` genau einmal, in der Reihenfolge seiner ersten Registrierung
- [ ] Dasselbe Behavior zweimal nur für einen Request-Typ registriert → es läuft pro `send` genau einmal
- [ ] Zwölf Behaviors → sie umschließen den Handler in Registrierungsreihenfolge, das erste außen
- [ ] Behaviors und Publish-Strategie `Concurrent` zusammen → `send` läuft durch die Behaviors, `publish` lässt die Handler gleichzeitig laufen
- [ ] Ein Behavior nach dem ersten Handler registriert → kompiliert nicht
- [ ] Ein Behavior verlangt mehr vom Request (z. B. `Clone`), ein Request-Typ erfüllt das nicht → `send` für diesen Typ kompiliert nicht
- [ ] Handler mit `Handler::transient` registriert und von einem Behavior umschlossen → ein `send` baut den Handler genau einmal, das Behavior läuft genau einmal um ihn herum
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
