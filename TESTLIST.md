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
- [x] Mediator mit Behaviors in einem Feld vom Typ `Mediator` gespeichert, ohne die Behaviors zu nennen → `send` darüber läuft durch die Behaviors und liefert die Antwort des Handlers · `mediator_with_behaviors_stored_as_plain_mediator_field_runs_behaviors`
- [x] (T11) Mediator mit Behaviors, ein `Handler::singleton` hält `WeakMediator` als Feld, ohne die Behaviors zu nennen → `send` über diesen `WeakMediator` läuft durch die Behaviors · `singleton_handler_with_plain_weak_mediator_field_runs_behaviors`
- [x] Mediator mit Stream-Behaviors in einem Feld vom Typ `Mediator` gespeichert, ohne die Behaviors zu nennen → `create_stream` darüber läuft durch die Stream-Behaviors und liefert die Items des Handlers · `mediator_with_stream_behaviors_stored_as_plain_mediator_field_runs_stream_behaviors`
- [x] Mediator mit einem Behavior in eine mit `tokio::spawn` gestartete Aufgabe verschoben, dort `send` → liefert die Antwort des Handlers, das Behavior lief · `send_inside_spawned_task_runs_behavior_and_returns_handler_response`
- [x] (T11) Handler bekommt den Mediator per Konstruktor als generischen `Sender`, Mediator in eine mit `tokio::spawn` gestartete Aufgabe verschoben, dort `send` → liefert die Antwort des Handlers · `generic_handler_with_injected_sender_runs_inside_spawned_task`
- [x] Mediator in eine mit `tokio::spawn` gestartete Aufgabe verschoben, dort `publish` → alle Handler laufen · `publish_inside_spawned_task_runs_all_handlers`
- [x] Mediator in eine mit `tokio::spawn` gestartete Aufgabe verschoben, dort einen Stream gelesen → er liefert die Elemente des Stream-Handlers · `stream_read_inside_spawned_task_yields_handler_items`
- [x] (T11) Zwei Threads schicken über Klone desselben Mediators gleichzeitig Requests an einen `Handler::singleton` → beide bekommen ihre Antwort, der Handler wird einmal gebaut · `two_threads_send_through_cloned_mediator_build_singleton_once`

## Mediator — Request-Scope

Quelle: Microsoft.Extensions.DependencyInjection `AddScoped`, `IServiceScopeFactory.CreateScope`, `ValidateScopes`, Factory-Registrierung `AddScoped(sp => …)`; MediatR `Mediator(IServiceProvider)` holt Handler aus dem Provider, mit dem er gebaut ist. Fachlich: ein Warenkorb pro Web-Anfrage, `AddToCart` liefert die Summe bisher; eine Bestellung pro Web-Anfrage sammelt Zeilen in einer `UnitOfWork`, die für die DB-Transaktion steht.

- [x] Warenkorb mit `Handler::scoped`, zwei `AddToCart` über denselben Scope aus `Mediator::scope` → das zweite liefert die Summe beider Artikel · `scoped_cart_sums_items_added_within_same_scope`
- [x] (T11) Warenkorb mit `Handler::scoped`, je ein `AddToCart` über zwei Scopes → jeder liefert nur seinen Artikel · `scoped_cart_starts_empty_in_each_new_scope`
- [x] Warenkorb mit `Handler::scoped`, `AddToCart` direkt über den Mediator ohne `scope` → `Err(SendError::NoScope)`, kein Panic · `scoped_cart_without_scope_gets_no_scope`
- [x] (T11) Handler mit `Handler::transient`, über einen Scope aufgerufen, schickt über den Mediator, den er bekommt, `AddToCart` an den Warenkorb → `AddToCart` trifft denselben Warenkorb wie ein `send` direkt über diesen Scope · `transient_handler_reaches_scoped_cart_of_its_scope`
- [x] (T11) Handler mit `Handler::singleton`, `send` über zwei Scopes und direkt über den Mediator → der Handler wird einmal gebaut · `singleton_handler_is_built_once_across_scopes_and_mediator`
- [x] (T11) Mediator mit einem Behavior, `AddToCart` über einen Scope an den Warenkorb mit `Handler::scoped` → das Behavior läuft, die Antwort kommt vom Warenkorb · `behavior_runs_around_scoped_cart_in_scope`
- [x] (T11) `Handler::singleton`, über einen Scope aufgerufen, schickt über seinen `WeakMediator` `AddToCart` an den `scoped`-Warenkorb → `Err(SendError::NoScope)`, kein Panic · `singleton_handler_sending_to_scoped_cart_gets_no_scope`
- [x] (T11) Scope fällt weg, der Mediator lebt → der Warenkorb wird freigegeben (sein `Drop` läuft) · `scoped_cart_is_released_with_its_scope`
- [ ] (T11) Ein Warenkorb, der den `WeakMediator` seines Scopes behalten hat, schickt `AddToCart`, nachdem der Scope weggefallen ist; der Mediator lebt → `Err(SendError::MediatorDropped)`, kein Panic
- [ ] Notification-Handler mit `Handler::scoped`, `publish` über einen Scope → der Handler läuft
- [ ] Notification-Handler mit `Handler::scoped`, `publish` direkt über den Mediator ohne `scope` → `Err(SendError::NoScope)`, kein Panic
- [ ] Stream-Handler mit `Handler::scoped`, `create_stream` über einen Scope → der Stream liefert die Elemente des Handlers
- [ ] Stream-Handler mit `Handler::scoped`, `create_stream` direkt über den Mediator ohne `scope` → der Stream liefert ein Element `Err(SendError::NoScope)` und endet, kein Panic
- [ ] Eigener Dienst `UnitOfWork` mit `Registration::scoped` über `MediatorBuilder::service` registriert, zwei Handler mit `Handler::scoped_from`, deren Bauanleitung `UnitOfWork` aus dem Provider holt, `AddOrderLine` und danach `CountOrderLines` über denselben Scope → `CountOrderLines` liefert 1
- [ ] (T11) Dieselben Registrierungen, `AddOrderLine` über einen Scope, `CountOrderLines` über einen zweiten → `CountOrderLines` liefert 0
- [ ] Handler mit `Handler::scoped_from`, dessen Bauanleitung `UnitOfWork` holt, `UnitOfWork` nicht registriert, `send` über einen Scope → `Err(SendError::NoDependency)`, kein Panic

## Provider-Crate `mediatr-provider`

Quelle: Lebensdauern von `Microsoft.Extensions.DependencyInjection` (`AddSingleton`, `AddScoped`, `AddTransient`, `TryAddEnumerable` als `unique_by`); MediatR `test/MediatR.Tests/MicrosoftExtensionsDI/TypeResolutionTests.cs` (`ShouldNotThrowWithMissingEnumerables`). Der Container registriert und baut Dienste; vom Mediator weiß er nichts. `ValidateScopes` ist immer an.

- [x] Dienst mit `instance` registriert → jede Abfrage beim Provider liefert dasselbe Objekt · `instance_returns_same_object_on_every_get`
- [x] Dienst nicht registriert → die Abfrage beim Provider liefert `Err(GetError::NoService)` · `get_without_registration_returns_no_service`
- [x] Zweimal `instance` für denselben Typ → die Abfrage liefert den zuletzt registrierten Dienst · `instance_twice_returns_last_registered_service`
- [x] Dienst mit `transient` registriert → die Bauanleitung läuft bei jeder Abfrage beim Provider neu · `transient_runs_recipe_on_every_get`
- [x] Dienst mit `singleton` registriert → die Bauanleitung läuft einmal, bei der ersten Abfrage beim Provider · `singleton_runs_recipe_once_on_first_get`
- [x] Dienst mit `scoped` registriert → die Bauanleitung läuft einmal pro Scope; ein neuer Scope baut neu · `scoped_runs_recipe_once_per_scope`
- [x] Dienst mit `singleton` registriert → ein neuer Scope liefert dasselbe Objekt wie der Provider · `singleton_returns_same_object_in_new_scope`
- [x] Bauanleitung fragt den Provider nach einem anderen registrierten Dienst → der gebaute Dienst enthält diesen Dienst · `recipe_gets_other_service_from_provider`
- [x] Bauanleitung fragt den Provider nach ihrem eigenen Typ → diese Abfrage liefert `Err(GetError::Cycle)`, kein Panic und keine Endlosschleife · `recipe_asking_for_own_type_gets_cycle`
- [x] Zwei Bauanleitungen fragen einander ab (A braucht B, B braucht A) → die innere Abfrage nach A liefert keinen Dienst, kein Panic und keine Endlosschleife · `mutually_dependent_recipes_get_nothing_for_inner_query`
- [x] Bauanleitung von `singleton` fragt nach ihrem eigenen Typ → diese Abfrage liefert keinen Dienst, kein Panic und keine Endlosschleife · `singleton_recipe_asking_for_own_type_gets_nothing`
- [x] Bauanleitung von `scoped` fragt, abgefragt in einem Scope, nach ihrem eigenen Typ → diese Abfrage liefert keinen Dienst, kein Panic und keine Endlosschleife · `scoped_recipe_asking_for_own_type_in_scope_gets_nothing`
- [x] Bauanleitung fragt zweimal nach ihrem eigenen Typ → beide Abfragen liefern keinen Dienst, kein Panic und keine Endlosschleife · `recipe_asking_twice_for_own_type_gets_nothing_both_times`
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
- [x] Zwei Dienste desselben Typs mit `instance` registriert → `all` liefert beide, in der Reihenfolge der Registrierung · `all_returns_every_instance_in_registration_order`
- [x] Dienst nicht registriert → `all` liefert eine leere Liste · `all_without_registration_returns_empty_list`
- [x] Dienst mit `transient` registriert → `all` baut ihn bei jeder Abfrage neu · `transient_all_runs_recipe_on_every_call`
- [x] Dienst mit `singleton` registriert → `all` liefert dasselbe Objekt wie `get` · `singleton_all_returns_same_object_as_get`
- [x] Dienst mit `scoped` registriert → `all` liefert im selben Scope dasselbe Objekt wie `get`; ein neuer Scope baut neu · `scoped_all_returns_same_object_as_get_within_scope`
- [x] Bauanleitung fragt mit `all` nach ihrem eigenen Typ → die Abfrage liefert `Err(GetError::Cycle)`, kein Panic und keine Endlosschleife · `recipe_asking_all_for_own_type_gets_cycle`
- [x] Bauanleitung fragt mit `all` nach ihrem eigenen Typ, die äußere Abfrage kommt über `get` → die innere Abfrage liefert keine Dienste, kein Panic und keine Endlosschleife · `recipe_asking_all_for_own_type_through_get_gets_nothing`
- [x] Zwei Bauanleitungen fragen einander mit `all` ab → die innere Abfrage liefert keine Dienste, kein Panic und keine Endlosschleife · `mutually_dependent_recipes_get_nothing_from_inner_all`
- [x] Zwei Bauanleitungen desselben Typs, die erste fragt mit `get` nach ihrem eigenen Typ, die äußere Abfrage kommt über `all` → die innere Abfrage liefert keinen Dienst · `first_of_two_recipes_asking_for_own_type_through_all_gets_nothing`
- [x] Zwei Dienste desselben Typs, die erste Bauanleitung fragt mit `get` nach ihrem eigenen Typ, der zweite ist eine `instance`, die äußere Abfrage kommt über `all` → die innere Abfrage liefert diese `instance`, kein `Err(GetError::Cycle)` · `first_recipe_asking_for_own_type_through_all_gets_last_instance`
- [x] Zwei Dienste desselben Typs, die erste Bauanleitung fragt mit `get` nach ihrem eigenen Typ, der zweite ist ein schon gebautes `singleton`, die äußere Abfrage kommt über `all` → die innere Abfrage liefert dieses Singleton, kein `Err(GetError::Cycle)` · `first_recipe_asking_for_own_type_through_all_gets_built_singleton`
- [x] Zwei Dienste desselben Typs, die erste Bauanleitung fragt mit `get` nach ihrem eigenen Typ, der zweite ist ein im selben Scope schon gebauter `scoped`-Dienst, die äußere Abfrage kommt in diesem Scope über `all` → die innere Abfrage liefert diesen Dienst, kein `Err(GetError::Cycle)` · `first_recipe_asking_for_own_type_through_all_in_scope_gets_built_scoped_service`
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
- [x] Provider in einen anderen Thread verschoben → `get` liefert dort den registrierten Dienst · `provider_moved_to_other_thread_returns_registered_service`
- [x] Bauanleitung eines `transient`-Diensts läuft auf einem Thread, ein zweiter Thread fragt währenddessen denselben Typ ab → der zweite bekommt den Dienst · `second_thread_gets_transient_service_while_first_thread_builds_it`
- [x] (T11) Zwei Threads fragen gleichzeitig ein `singleton` ab → die Bauanleitung läuft einmal, beide bekommen dasselbe Objekt · `two_threads_get_same_singleton_built_once`
- [x] (T11) Zwei Threads fragen im selben Scope gleichzeitig denselben `scoped`-Dienst ab → beide bekommen dasselbe Objekt · `two_threads_get_same_scoped_service_in_one_scope`
- [x] Zwei `singleton`-Bauanleitungen fragen einander ab, zwei Threads starten sie gleichzeitig → `get` kommt zurück, kein Warten für immer · `mutually_dependent_singletons_on_two_threads_both_return`
- [x] Zwei `singleton`-Bauanleitungen fragen einander ab, zwei Threads starten sie gleichzeitig → genau eine der beiden inneren Abfragen liefert `Err(GetError::Cycle)` · `mutually_dependent_singletons_on_two_threads_exactly_one_inner_query_gets_cycle`
- [x] Drei `singleton`-Bauanleitungen fragen einander im Kreis ab, drei Threads starten sie gleichzeitig → jedes `get` kommt zurück, genau eine der drei inneren Abfragen liefert `Err(GetError::Cycle)` · `three_singletons_in_a_ring_on_three_threads_all_return_and_exactly_one_inner_query_gets_cycle`
- [x] Zwei `singleton`-Registrierungen desselben Typs, zwei Threads bauen sie gleichzeitig, die erste fragt ein Singleton ab, das ein dritter Thread baut und das auf die erste wartet → genau eine der beiden inneren Abfragen liefert `Err(GetError::Cycle)`, kein Warten für immer · `same_type_singletons_on_two_threads_with_third_thread_waiting_on_first_get_one_cycle`
- [x] Dienst mit `scoped` registriert, Abfrage beim Provider selbst statt in einem Scope → die Abfrage liefert `Err(GetError::ScopedFromRoot)` · `scoped_service_from_provider_itself_gets_scoped_from_root`
- [x] (T11) Bauanleitung von `singleton` fragt nach einem `scoped`-Dienst, erste Abfrage kommt aus einem Scope → diese Abfrage liefert `Err(GetError::ScopedFromRoot)` · `singleton_recipe_asking_for_scoped_service_gets_scoped_from_root`
- [x] (T11) Bauanleitung von `transient` fragt, abgefragt in einem Scope, nach einem `scoped`-Dienst → sie bekommt den Dienst dieses Scopes · `transient_recipe_in_scope_gets_scoped_service_of_scope`
- [x] (T11) Dienst mit `scoped` registriert, `all` beim Provider selbst statt in einem Scope → `all` liefert `Err(GetError::ScopedFromRoot)` · `scoped_all_from_provider_itself_gets_scoped_from_root`
- [x] Bauanleitung von `transient` fragt nach einem nicht registrierten Dienst und gibt den Fehler mit `?` weiter → die äußere Abfrage liefert `Err(GetError::NoDependency)` · `recipe_asking_for_unregistered_service_gets_no_dependency`
- [x] (T11) Bauanleitung gibt das `Err(GetError::Cycle)` einer inneren Abfrage weiter → die äußere Abfrage liefert `Err(GetError::Cycle)`, nicht `NoDependency` · `recipe_passing_on_cycle_keeps_cycle`
- [x] Bauanleitung, über `all` abgefragt, fragt nach einem nicht registrierten Dienst und gibt den Fehler weiter → `all` liefert `Err(GetError::NoDependency)` · `all_with_recipe_asking_for_unregistered_service_gets_no_dependency`
- [x] Bauanleitung von `scoped` fragt nach einem nicht registrierten Dienst und gibt den Fehler weiter → die Abfrage im Scope liefert `Err(GetError::NoDependency)` · `scoped_recipe_asking_for_unregistered_service_gets_no_dependency`
- [x] Bauanleitung von `singleton` fragt nach einem nicht registrierten Dienst und gibt den Fehler weiter → die Abfrage liefert `Err(GetError::NoDependency)`, und die nächste Abfrage baut neu · `singleton_recipe_asking_for_unregistered_service_gets_no_dependency_and_builds_again`
- [x] `ProviderBuilder` ohne Registrierung mit `unique_by` → `has_unique` liefert `false` · `has_unique_without_unique_registration_returns_false`
- [x] Registrierung mit `unique_by` für einen Typ und einen Schlüsseltyp → `has_unique` für genau dieses Paar liefert `true` · `has_unique_after_unique_registration_returns_true`
- [x] (T11) Registrierung mit `unique_by` für einen Typ und einen Schlüsseltyp → `has_unique` für einen anderen Typ mit demselben Schlüsseltyp liefert `false` · `has_unique_for_other_type_with_same_key_returns_false`
- [x] Registrierung mit `unique_by` für einen Typ und einen Schlüsseltyp → `has_unique` für denselben Typ mit einem anderen Schlüsseltyp liefert `false` · `has_unique_for_same_type_with_other_key_returns_false`

## Notification — `publish`

Quelle: MediatR `test/MediatR.Tests/PublishTests.cs`, `ExceptionTests.cs`, `MicrosoftExtensionsDI/Issue1118Tests.cs`, `MicrosoftExtensionsDI/DuplicateAssemblyResolutionTests.cs`; Standard-Publisher `ForeachAwaitPublisher` (nacheinander, erster Fehler stoppt). Doppelte Handler-Typen: `NotificationHandlerWrapper` behält mit `GroupBy(GetType).First()` den zuerst registrierten, anders als `send`. Andere Publisher stehen unter „Notification — Notification-Publisher“.

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

## Notification — Notification-Publisher

Quelle: MediatR `test/MediatR.Tests/PublishTests.cs`, `NotificationPublisherTests.cs`, `MicrosoftExtensionsDI/NotificationPublisherTests.cs`; `src/MediatR/NotificationPublishers/TaskWhenAllPublisher.cs` (startet alle Handler, meldet den ersten Fehler erst, wenn alle fertig sind). Default bleibt „nacheinander“ wie `ForeachAwaitPublisher`, wie in MediatR. „Alle gleichzeitig“ gibt es nur bewusst gewählt; wer eine feste Reihenfolge der Handler braucht, bleibt beim Default.

- [x] Mediator mit eigenem Notification-Publisher, der die Handler in umgekehrter Reihenfolge aufruft, Notification mit zwei Handlern → beide laufen, in der Reihenfolge des eigenen Publishers · `custom_publisher_runs_both_handlers_in_its_own_order`
- [x] Mediator mit eigenem Notification-Publisher, Request mit registriertem Handler → `send` liefert die Antwort des Handlers · `custom_publisher_send_returns_handler_response`
- [x] Mediator mit eigenem Notification-Publisher, Handler mit `Handler::singleton` schickt über den `WeakMediator`, den er bekommt, einen Request → `send` liefert die Antwort von dessen Handler · `custom_publisher_singleton_handler_sends_through_received_weak_mediator`
- [x] Mediator und `WeakMediator` mit eigenem Notification-Publisher, der kein `Clone` hat → lassen sich klonen, und der Klon erreicht dieselben Handler · `custom_publisher_without_clone_mediator_and_weak_mediator_clone_reaches_same_handlers`
- [x] Handler mit `Handler::singleton` in einem Mediator mit eigenem Notification-Publisher klont den `WeakMediator`, den er bekommt, und meldet über Original und Klon je eine Notification → beide erreichen dieselben Handler · `custom_publisher_singleton_handler_publishes_through_weak_mediator_and_its_clone`
- [x] Mediator, `WeakMediator` und `MediatorBuilder` mit eigenem Notification-Publisher, der kein `Debug` hat, mit `{:?}` formatiert → die Ausgabe nennt den Publisher · `custom_publisher_without_debug_mediator_weak_mediator_and_builder_debug_names_publisher`
- [x] `MediatorBuilder` mit eigenem Notification-Publisher, der kein `Clone` hat → lässt sich klonen, und beide bauen Mediatoren, die dieselben Handler erreichen · `custom_publisher_without_clone_builder_clone_builds_mediators_reaching_same_handlers`
- [x] Mediator mit eigenem Notification-Publisher, Handler mit `Handler::singleton` meldet über den `WeakMediator`, nachdem der Mediator weggefallen ist → `publish` liefert `Err(SendError::MediatorDropped)`, kein Panic · `custom_publisher_singleton_handler_publishing_after_mediator_dropped_gets_mediator_dropped`
- [x] Mediator mit eigenem Notification-Publisher, Handler mit `Handler::transient` meldet über den Mediator, den er bekommt, eine andere Notification → auch deren Handler laufen in der Reihenfolge des eigenen Publishers · `custom_publisher_transient_handler_publishes_through_received_mediator_in_its_order`
- [x] Mediator mit eigenem Notification-Publisher, Handler mit `Handler::singleton` meldet über den `WeakMediator`, den er bekommt, eine andere Notification → auch deren Handler laufen in der Reihenfolge des eigenen Publishers · `custom_publisher_singleton_handler_publishes_through_received_weak_mediator_in_its_order`
- [x] Publisher „alle gleichzeitig“, zwei Handler, die beide warten (`await`) → der zweite Handler startet, bevor der erste fertig ist · `when_all_publisher_starts_second_waiting_handler_before_first_finished`
- [x] Publisher „alle gleichzeitig“, zwei Handler, die gleich lange warten → sie werden in der Reihenfolge der Registrierung fertig · `when_all_publisher_equally_waiting_handlers_finish_in_registration_order`
- [x] Publisher „alle gleichzeitig“, Notification mit drei Handlern, keiner scheitert → `publish` liefert `Ok(())`, jeder Handler lief genau einmal · `when_all_publisher_runs_each_of_three_handlers_once_and_returns_ok`
- [x] Publisher „alle gleichzeitig“, der erste von zwei Handlern liefert einen Fehler → der zweite Handler läuft trotzdem, `publish` liefert den Fehler des ersten · `when_all_publisher_runs_second_handler_after_first_fails_and_returns_first_error`
- [x] Publisher „alle gleichzeitig“, beide von zwei Handlern liefern einen Fehler → `publish` liefert den Fehler des zuerst registrierten · `when_all_publisher_both_handlers_fail_returns_error_of_first_registered`
- [x] Publisher „alle gleichzeitig“, beide von zwei Handlern liefern einen Fehler, der zuerst registrierte erst nach einem `await` → `publish` liefert trotzdem den Fehler des zuerst registrierten · `when_all_publisher_both_fail_first_after_await_returns_error_of_first_registered`
- [x] Publisher „alle gleichzeitig“, nur der zweite von zwei Handlern liefert einen Fehler → der erste lief, `publish` liefert den Fehler des zweiten · `when_all_publisher_only_second_handler_fails_runs_first_and_returns_second_error`

## Pipeline — Behaviors

Quelle: MediatR `test/MediatR.Tests/PipelineTests.cs`, `MicrosoftExtensionsDI/PipelineTests.cs`, `MicrosoftExtensionsDI/PipeLineMultiCallToConstructorTest.cs`, `SendTests.cs` (Timeout-Tests); `docs/mediatr-concepts.md` (Behavior bricht ab). Wie bei MediatR gibt es Behaviors für alle Request-Typen und Behaviors nur für einen; man registriert beide Arten in freier Reihenfolge vor dem ersten Handler, das zuerst registrierte liegt außen. Ein mehrfach registriertes Behavior läuft wie bei `TryAddEnumerable` nur einmal, an der Stelle seiner ersten Registrierung. Anders als MediatR überspringt `send` kein Behavior, das mehr vom Request verlangt; es kompiliert dann nicht (Entscheidung Markus).

- [x] Zwei Behaviors, ein Request → das zuerst registrierte läuft außen: außen-vorher, innen-vorher, Handler, innen-nachher, außen-nachher; `send` liefert die Antwort des Handlers · `two_behaviors_wrap_handler_first_registered_outermost`
- [x] Behavior klont den nächsten Schritt und ruft ihn nach einem Fehler des Handlers noch einmal auf → der Handler läuft zweimal, `send` liefert die Antwort des zweiten Laufs · `behavior_retries_cloned_next_after_handler_error_and_returns_second_response`
- [x] Zwei Behaviors, Request ohne Antwort (Antworttyp `()`) → sie umschließen den Handler in derselben Reihenfolge, das zuerst registrierte außen · `two_behaviors_wrap_unit_response_handler_first_registered_outermost`
- [x] Zwei Behaviors, Requests zweier Typen mit je eigenem Handler → jeder Request läuft durch beide Behaviors, das zuerst registrierte außen; `send` liefert jeweils die Antwort seines Handlers · `two_behaviors_wrap_handlers_of_every_request_type_first_registered_outermost`
- [x] Ein Behavior für alle Request-Typen, eins nur für einen, noch eins für alle, in dieser Reihenfolge registriert → ein Request dieses Typs läuft durch alle drei in Registrierungsreihenfolge, das erste außen; ein Request eines anderen Typs nur durch die zwei allgemeinen; beide bekommen die Antwort ihres Handlers · `behavior_between_two_open_behaviors_wraps_only_its_requests_in_registration_order`
- [x] Zuerst ein Behavior nur für einen Request-Typ ohne Antwort (Antworttyp `()`), dann eins für alle registriert → beide laufen in Registrierungsreihenfolge um den Handler, das erste außen; `send` liefert `Ok(())` · `unit_response_behavior_then_open_behavior_wrap_handler_first_registered_outermost`
- [x] Zwei Behaviors, jedes dreimal abwechselnd registriert → jedes läuft pro `send` genau einmal, in der Reihenfolge seiner ersten Registrierung · `two_behaviors_registered_three_times_alternately_run_once_in_first_registration_order`
- [x] Dasselbe Behavior zweimal nur für einen Request-Typ registriert → es läuft pro `send` genau einmal · `same_behavior_registered_twice_for_one_request_type_runs_once`
- [x] Dasselbe Behavior einmal für alle Request-Typen und einmal nur für einen registriert → ein Request dieses Typs läuft durch beide · `same_behavior_registered_for_all_and_for_one_request_type_runs_twice_for_that_type`
- [x] Dasselbe Behavior zuerst nur für einen Request-Typ, dann für alle registriert → ein Request dieses Typs läuft durch beide · `same_behavior_registered_for_one_request_type_then_for_all_runs_twice_for_that_type`
- [x] Dasselbe Behavior einmal für alle Request-Typen und einmal nur für einen registriert → ein Request eines anderen Typs läuft nur einmal hindurch · `same_behavior_registered_for_all_and_for_one_request_type_runs_once_for_another_type`
- [x] Zwölf Behaviors → sie umschließen den Handler in Registrierungsreihenfolge, das erste außen · `twelve_behaviors_wrap_handler_first_registered_outermost`
- [x] Behaviors und `WhenAllPublisher` zusammen → `send` läuft durch die Behaviors, `publish` lässt die Handler gleichzeitig laufen · `behaviors_with_when_all_publisher_wrap_send_and_publish_starts_handlers_together`
- [x] Ein Behavior nach dem ersten Handler registriert → kompiliert nicht · `behavior_registered_after_first_handler_does_not_compile`
- [x] Ein Behavior nach dem ersten Notification-Handler (`subscribe`) registriert → kompiliert nicht · `behavior_registered_after_first_notification_handler_does_not_compile`
- [x] Ein Behavior verlangt mehr vom Request (z. B. `Clone`), ein Request-Typ mit Handler erfüllt das nicht → kompiliert nicht · `behavior_requiring_clone_rejects_non_clone_request_at_compile_time`
- [x] Ein Behavior verlangt `Clone`, ein Request-Typ ohne Handler erfüllt das nicht → `send` für diesen Typ liefert `Err(SendError::NoHandler)`, kein Behavior läuft · `behavior_requiring_clone_with_non_clone_request_without_handler_returns_no_handler`
- [x] Handler mit `Handler::transient` registriert und von einem Behavior umschlossen → ein `send` baut den Handler genau einmal, das Behavior läuft genau einmal um ihn herum · `transient_handler_wrapped_by_behavior_is_built_once_per_send_and_wrapped_once`
- [x] Behavior ruft den nächsten Schritt nicht auf und liefert selbst eine Antwort → `send` liefert diese Antwort, der Handler läuft nicht · `behavior_answering_without_next_returns_its_response_and_handler_does_not_run`
- [x] Behavior setzt eine Frist, der Handler eines Requests mit Antwort braucht länger → `send` liefert `Err` statt einer Antwort, der Handler wird nicht fertig · `behavior_deadline_shorter_than_handler_returns_err_and_handler_does_not_finish`
- [x] Behavior setzt eine Frist, der Handler eines Requests ohne Antwort (Antworttyp `()`) braucht länger → `send` liefert `Err`, der Handler wird nicht fertig · `behavior_deadline_shorter_than_unit_response_handler_returns_err_and_handler_does_not_finish`

## Pipeline — Vor- und Nachverarbeiter

Quelle: MediatR `test/MediatR.Tests/Pipeline/RequestPreProcessorTests.cs`, `Pipeline/RequestPostProcessorTests.cs`, `MicrosoftExtensionsDI/PipelineTests.cs`. Vor- und Nachverarbeiter laufen außerhalb aller Behaviors; Nachverarbeiter laufen in Registrierungsreihenfolge, nicht umgekehrt.

- [x] Request mit registriertem Vorverarbeiter → er läuft vor dem Handler und bekommt den Request; `send` liefert die Antwort des Handlers · `pre_processor_runs_before_handler_with_request_and_send_returns_handler_response`
- [x] Request mit registriertem Nachverarbeiter → er läuft nach dem Handler und bekommt Request und Antwort; `send` liefert die Antwort des Handlers · `post_processor_runs_after_handler_with_request_and_response_and_send_returns_handler_response`
- [x] Zwei Vorverarbeiter nur für einen Request-Typ und zwei für alle, ebenso vier Nachverarbeiter → alle Vorverarbeiter laufen in Registrierungsreihenfolge vor dem Handler, alle Nachverarbeiter in Registrierungsreihenfolge danach · `pre_and_post_processors_for_one_and_all_request_types_run_in_registration_order`
- [x] Behaviors, Vor- und Nachverarbeiter teils für alle Request-Typen, teils nur für einen → dieser Request läuft durch Vorverarbeiter, Behaviors (außen nach innen), Handler, Nachverarbeiter; ein anderer Request nur durch die allgemeinen, in derselben Reihenfolge · `processors_run_outside_behaviors_and_other_request_only_through_general_ones`
- [x] Handler scheitert, ein Nachverarbeiter ist registriert → er läuft nicht; `send` liefert den Fehler des Handlers · `failing_handler_skips_post_processor_and_send_returns_handler_error`
- [x] Nachverarbeiter registriert, Request-Typ mit Handler ohne `Clone` → kompiliert nicht · `post_processor_rejects_non_clone_request_at_compile_time`
- [x] Derselbe Vorverarbeiter-Typ zweimal registriert → er läuft pro `send` einmal · `same_pre_processor_type_registered_twice_runs_once_per_send`
- [x] Vorverarbeiter nach dem ersten Handler registriert → kompiliert nicht · `pre_processor_registered_after_first_handler_does_not_compile`
- [x] Nachverarbeiter nach dem ersten Handler registriert → kompiliert nicht · `post_processor_registered_after_first_handler_does_not_compile`
- [x] Vorverarbeiter scheitert → der Handler läuft nicht; `send` liefert genau den Fehler des Vorverarbeiters · `failing_pre_processor_skips_handler_and_send_returns_its_error`
- [x] Nachverarbeiter scheitert → `send` liefert genau den Fehler des Nachverarbeiters statt der Antwort des Handlers · `failing_post_processor_replaces_handler_response_with_its_error`

## Pipeline — Fehler-Handler und Fehler-Aktionen

Quelle: MediatR `test/MediatR.Tests/Pipeline/RequestExceptionHandlerTests.cs`, `Pipeline/RequestExceptionActionTests.cs`, `MicrosoftExtensionsDI/PipelineTests.cs`. Ein Fehler-Handler kann eine Ersatzantwort setzen; eine Fehler-Aktion reagiert nur und läuft, wenn kein Fehler-Handler eine Ersatzantwort gesetzt hat (Standard `ApplyForUnhandledExceptions`). MediatR wählt nach der Fehler-Basisklasse; ohne Vererbung laufen sie in Registrierungsreihenfolge.

- [x] Handler scheitert, eine Fehler-Aktion ist registriert → die Aktion läuft, `send` liefert trotzdem genau den Fehler des Handlers · `error_action_runs_on_handler_error_and_send_returns_handler_error`
- [x] Handler scheitert, eine Fehler-Aktion ist registriert → sie läuft pro `send` genau einmal · `error_action_runs_once_per_send_on_handler_error`
- [x] Handler scheitert, zwei Fehler-Aktionen sind registriert → beide laufen, `send` liefert den Fehler des Handlers · `two_error_actions_both_run_on_handler_error_and_send_returns_handler_error`
- [x] Handler scheitert, zwei Fehler-Handler, der erste setzt eine Ersatzantwort → `send` liefert `Ok` mit dieser Ersatzantwort, der zweite Fehler-Handler läuft nicht · `first_error_handler_replacement_is_returned_and_second_error_handler_does_not_run`
- [x] Handler scheitert, der Fehler-Handler setzt keine Ersatzantwort → `send` liefert den ursprünglichen Fehler · `error_handler_without_replacement_lets_send_return_original_error`
- [x] Handler scheitert, der Fehler-Handler scheitert selbst → `send` liefert den Fehler des Fehler-Handlers, nicht den ursprünglichen · `failing_error_handler_makes_send_return_its_error_instead_of_original`
- [x] Handler scheitert, ein Fehler-Handler ohne Ersatzantwort ist registriert → er läuft pro `send` genau einmal · `error_handler_without_replacement_runs_once_per_send_on_handler_error`
- [x] Handler scheitert, ein Fehler-Handler setzt eine Ersatzantwort, zusätzlich ist eine Fehler-Aktion registriert → `send` liefert die Ersatzantwort, die Fehler-Aktion läuft nicht · `error_handler_replacement_is_returned_and_error_action_does_not_run`
- [x] Handler scheitert, ein Fehler-Handler ohne Ersatzantwort und eine Fehler-Aktion sind registriert → beide laufen, `send` liefert den Fehler des Handlers · `error_handler_without_replacement_and_error_action_both_run_and_send_returns_handler_error`
- [x] Ein Behavior scheitert, bevor der Handler läuft, ein Fehler-Handler ohne Ersatzantwort und eine Fehler-Aktion sind registriert → beide laufen, `send` liefert den Fehler des Behaviors · `failing_behavior_before_handler_runs_error_handler_and_error_action_and_send_returns_its_error`
- [x] Handler scheitert, die Fehler-Aktion scheitert selbst → `send` liefert genau den Fehler der Aktion statt den des Handlers · `failing_error_action_makes_send_return_its_error_instead_of_handler_error`
- [x] Handler hat Erfolg, eine Fehler-Aktion ist registriert → sie läuft nicht, `send` liefert die Antwort des Handlers · `successful_handler_skips_error_action_and_send_returns_its_response`
- [x] Ein Vorverarbeiter scheitert, eine Fehler-Aktion ist registriert → sie läuft, `send` liefert den Fehler des Vorverarbeiters · `failing_pre_processor_runs_error_action_and_send_returns_its_error`
- [x] Ein Nachverarbeiter scheitert, eine Fehler-Aktion ist registriert → sie läuft, `send` liefert den Fehler des Nachverarbeiters · `failing_post_processor_runs_error_action_and_send_returns_its_error`
- [x] Fehler-Aktion nur für einen Request-Typ registriert, Requests dieses und eines anderen Typs scheitern → sie läuft nur für ihren Typ; für den anderen liefert `send` dessen Fehler · `error_action_for_one_request_type_runs_only_for_that_type`
- [x] Handler scheitert, zwei Fehler-Handler, der erste reicht einen eigenen Fehler weiter → der zweite bekommt diesen Fehler statt des ursprünglichen · `second_error_handler_gets_error_passed_on_by_first_instead_of_original`
- [x] Handler hat Erfolg, ein Fehler-Handler ist registriert → er läuft nicht, `send` liefert die Antwort des Handlers · `successful_handler_skips_error_handler_and_send_returns_its_response`
- [x] Fehler-Handler nur für einen Request-Typ registriert, Requests dieses und eines anderen Typs scheitern → er läuft nur für seinen Typ; für den anderen liefert `send` dessen Fehler · `error_handler_for_one_request_type_runs_only_for_that_type`
- [x] Derselbe Fehler-Handler-Typ zweimal registriert → er läuft pro `send` einmal · `same_error_handler_type_registered_twice_runs_once_per_send`
- [x] Derselbe Fehler-Aktions-Typ zweimal registriert → sie läuft pro `send` einmal · `same_error_action_type_registered_twice_runs_once_per_send`
- [x] Fehler-Handler registriert, Request-Typ mit Handler ohne `Clone` → kompiliert nicht · `error_handler_rejects_non_clone_request_at_compile_time`
- [x] Fehler-Aktion registriert, Request-Typ mit Handler ohne `Clone` → kompiliert nicht · `error_action_rejects_non_clone_request_at_compile_time`
- [x] Fehler-Handler nach dem ersten Handler registriert → kompiliert nicht · `error_handler_registered_after_first_handler_does_not_compile`
- [x] Fehler-Aktion nach dem ersten Handler registriert → kompiliert nicht · `error_action_registered_after_first_handler_does_not_compile`

## Stream-Request — Stream anfordern

Quelle: MediatR `test/MediatR.Tests/CreateStreamTests.cs`; `src/MediatR/Wrappers/StreamRequestHandlerWrapper.cs` (der Stream ist träge: der Handler läuft erst beim Lesen).

- [x] Stream-Request mit registriertem Stream-Handler → der Stream liefert genau die Elemente des Handlers und endet danach · `create_stream_yields_exactly_handler_items_then_ends`
- [x] Handler bekommt einen Sender per Konstruktor und fordert darüber einen Stream an → er bekommt die Elemente des Stream-Handlers · `handler_requests_stream_through_constructor_sender_and_gets_handler_items`
- [x] Stream-Request ohne registrierten Stream-Handler → der Stream liefert einen Fehler, kein Panic · `create_stream_without_handler_yields_no_handler_error`
- [x] Zwei Stream-Handler nacheinander für denselben Stream-Request-Typ registriert → der Stream liefert die Elemente des zuletzt registrierten · `register_stream_twice_keeps_last_stream_handler`
- [x] Stream angefordert, aber nie gelesen → der Stream-Handler läuft nicht · `unread_stream_does_not_run_stream_handler`

## Stream-Request — Stream-Behaviors

Quelle: MediatR `test/MediatR.Tests/StreamPipelineTests.cs`, `Pipeline/Streams/StreamPipelineBehaviorTests.cs`, `MicrosoftExtensionsDI/StreamPipelineTests.cs`. Das „nachher“ eines Stream-Behaviors läuft erst nach dem letzten Element.

- [x] Zwei Stream-Behaviors für einen Stream-Request-Typ → die Elemente des Handlers kommen unverändert an; Reihenfolge: erstes vorher, zweites vorher, Handler, zweites nachher, erstes nachher · `two_stream_behaviors_for_one_type_wrap_handler_in_registration_order_items_unchanged`
- [x] Zwei Stream-Behaviors für alle Stream-Request-Typen → sie umschließen den Handler, das zuerst registrierte außen; die Elemente kommen unverändert an · `two_open_stream_behaviors_wrap_handler_first_registered_outside_items_unchanged`
- [x] Zwei Stream-Behaviors für alle Stream-Request-Typen und ein drittes nur für einen → bei diesem Typ laufen alle drei in Registrierungsreihenfolge um den Handler, bei einem anderen Stream-Request-Typ nur die zwei allgemeinen · `stream_behavior_for_one_type_joins_general_ones_in_registration_order_other_type_gets_only_general`
- [x] Handler liefert drei Elemente, ein Stream-Behavior fügt vorne und hinten je eins an → der Stream liefert genau fünf Elemente in dieser Reihenfolge · `stream_behavior_adding_one_item_before_and_after_three_handler_items_yields_five_in_order`
- [x] Derselbe Stream-Behavior-Typ zweimal registriert → er läuft pro Stream einmal · `same_stream_behavior_type_registered_twice_runs_once_per_stream`
- [x] Stream-Behavior nach dem ersten Handler registriert → kompiliert nicht · `stream_behavior_registered_after_first_handler_does_not_compile`
- [x] Ein Behavior für alle Request-Typen ist registriert, ein Stream wird gelesen → der Stream läuft nicht durch das Behavior · `behavior_for_all_request_types_does_not_wrap_stream`
- [x] Ein Stream-Behavior für alle Stream-Request-Typen ist registriert, ein Request wird gesendet → `send` läuft nicht durch das Stream-Behavior · `stream_behavior_for_all_stream_request_types_does_not_wrap_send`
- [x] Ein Behavior nur für einen Request-Typ ist registriert, ein Stream wird gelesen → der Stream läuft nicht durch das Behavior · `behavior_for_one_request_type_does_not_wrap_stream`
- [x] Ein Stream-Behavior nur für einen Stream-Request-Typ ist registriert, ein Request wird gesendet → `send` läuft nicht durch das Stream-Behavior · `stream_behavior_for_one_stream_request_type_does_not_wrap_send`
- [x] Stream angefordert, aber nie gelesen → kein Stream-Behavior läuft · `unread_stream_runs_no_stream_behavior`
- [x] Ein Stream-Behavior ruft den nächsten Schritt auf, liest dessen Stream aber nicht → der Stream-Handler läuft nicht · `stream_behavior_not_reading_next_stream_keeps_stream_handler_from_running`
