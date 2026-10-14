# Verworfene MediatR-Tests

MediatR-Tests unter `test/`, die keinen Punkt in `TESTLIST.md` bekommen, weil sie nur C#-Technik prüfen. Alle anderen Tests stehen dort als Punkt. Stand: MediatR `main`, Oktober 2026.

## Lizenzprüfung (bewusst nicht portiert)

- `Licensing/LicenseAccessorTests.cs`: alle 2 Tests.
- `Licensing/LicenseKeyEnvironmentVariableTests.cs`: alle 8 Tests.
- `Licensing/LicenseValidationBackgroundTests.cs`: `License_validation_runs_off_the_construction_thread`.
- `Licensing/LicenseValidatorTests.cs`: alle 10 Tests (2 davon schon in MediatR mit `Skip`).
- `ServiceFactoryTests.cs`: `Should_not_throw_with_manual_registration`, `Should_throw_when_missing_required_configuration` (Konfiguration nur für die Lizenzprüfung; der Builder kennt keine fehlende Konfiguration).

## Assembly-Scan und Sichtbarkeit

Rust registriert jeden Handler ausdrücklich. Sichtbarkeit (`internal`, `private`) prüft der Compiler. Das Verhalten dahinter („registrierter Handler antwortet“, „mehrere Notification-Handler laufen“) steht in `TESTLIST.md`.

- `MicrosoftExtensionsDI/AssemblyResolutionTests.cs`: `ShouldResolveRequestHandler`, `ShouldResolveInternalHandler`, `ShouldResolveNotificationHandlers`, `ShouldResolveStreamHandlers`, `ShouldRequireAtLeastOneAssembly`, `ShouldNotThrowWhenAssemblyThrowsReflectionTypeLoadException`.
- `MediatR.DependencyInjectionTests/Abstractions/BaseAssemblyResolutionTests.cs`: alle 11 Tests (laufen dort in 6 fremden DI-Containern).
- `MicrosoftExtensionsDI/TypeEvaluatorTests.cs`: `ShouldOnlyResolveIncludedRequestHandlers` (Scan-Filter).
- `MicrosoftExtensionsDI/PipelineTests.cs`: `Should_auto_register_processors_when_configured_including_all_concrete_types`.
- `MicrosoftExtensionsDI/TypeResolutionTests.cs`: `ShouldResolveRequestHandler`, `ShouldResolveVoidRequestHandler`, `ShouldResolveNotificationHandlers`.
- `MicrosoftExtensionsDI/CustomMediatorTests.cs`: `ShouldResolveRequestHandler`, `ShouldResolveNotificationHandlers`.

## Offene generische Typen zur Laufzeit

Rust legt Generics beim Kompilieren fest. Jede konkrete Ausprägung ist ein normaler Handler.

- `SendTests.cs`: `Should_resolve_generic_handler`, `Should_resolve_generic_void_handler`, `Should_resolve_multiple_type_parameter_generic_handler`, `Should_resolve_closed_handler_if_defined`, `Should_resolve_open_handler_if_not_defined`.
- `GenericRequestHandlerTests.cs`: alle 8 Tests (Grenzen und Zeitlimit beim Schließen offener Generics; zwei Tests prüfen nur eine Test-Hilfsfunktion).
- `MicrosoftExtensionsDI/AssemblyResolutionTests.cs`: `ShouldResolveGenericVoidRequestHandler`, `ShouldResolveGenericReturnTypeRequestHandler`, `ShouldResolveGenericPingRequestHandler`, `ShouldResolveVoidGenericPingRequestHandler`.
- `MicrosoftExtensionsDI/PipelineTests.cs`: `Should_throw_when_adding_non_open_behavior`, `Should_throw_when_adding_non_open_stream_behavior`, `Should_throw_when_adding_random_generic_type_as_open_behavior`, `Should_apply_open_behavior_with_nested_generic_response_type`.
- `PipelineTests.cs`, `StreamPipelineTests.cs`: je `Should_handle_concrete_and_open_generics` (in MediatR mit `Skip`; das Verhalten deckt `Should_handle_constrained_generics` ab).

## `object`-Überladungen und Laufzeit-Typ

`send` und `publish` kennen den Nachrichtentyp beim Kompilieren.

- `SendTests.cs`: `Should_resolve_main_handler_via_dynamic_dispatch`, `Should_resolve_main_void_handler_via_dynamic_dispatch`, `Should_resolve_main_handler_by_given_interface`.
- `PublishTests.cs`: `Should_resolve_main_handler_when_object_is_passed`, `Should_resolve_handlers_given_interface`.
- `CreateStreamTests.cs`: `Should_resolve_main_handler_via_dynamic_dispatch`.
- `ExceptionTests.cs`: `Should_throw_argument_exception_for_publish_when_request_is_not_notification`, `Should_throw_exception_for_non_generic_send_when_exception_occurs`, `Should_throw_exception_for_non_request_send`.
- `GenericTypeConstraintsTests.cs`: `Should_Resolve_Void_Return_Request` (prüft die Interface-Hierarchie per Reflection).
- `UnitTests.cs`: `Should_be_equal`, `Should_compareto_value_as_zero`.

## `null`

Rust hat kein `null`; der Typ verlangt einen echten Wert.

- `SendTests.cs`: `Should_raise_execption_on_null_request`.
- `CreateStreamTests.cs`: `Should_raise_execption_on_null_request`, `Should_raise_execption_on_null_request_via_dynamic_dispatch`.
- `ExceptionTests.cs`: `Should_throw_argument_exception_for_send_when_request_is_null`, `Should_throw_argument_exception_for_void_send_when_request_is_null`, `Should_throw_argument_exception_for_publish_when_request_is_null`, `Should_throw_argument_exception_for_publish_when_request_is_null_object`.

## Der C#-Typ `Unit`

In Rust ist das `()`; Gleichheit, Hash, Ordnung und Ausgabe kommen aus std.

- `UnitTests.cs`: `Should_be_equal_to_each_other`, `Should_be_equitable`, `Should_tostring`, `Should_compareto_as_zero`.

## Vererbung und Kontravarianz

Rust hat keine Vererbung. Eine Nachricht hat genau ihre eigenen Handler.

- `PublishTests.cs`: `Should_override_with_sequential_firing` (überschreibt `PublishCore` in einer Unterklasse; die eigene Strategie steht als Punkt in „Notification — Publish-Strategie“).
- `NotificationHandlerTests.cs`: `Should_call_abstract_handle_method`.
- `MicrosoftExtensionsDI/Issue1118Tests.cs`: `Publishing_DerivedEvent_Should_Call_BaseEventHandler_ExactlyOnce`.

## Details der Microsoft-DI-Registrierung

Bei uns gibt es nur einen Weg: den Builder. Er liefert immer einen Mediator.

- `MicrosoftExtensionsDI/AssemblyResolutionTests.cs`, `TypeResolutionTests.cs`, `TypeEvaluatorTests.cs`, `CustomMediatorTests.cs`: je `ShouldResolveMediator`.
- `MicrosoftExtensionsDI/TypeResolutionTests.cs`: `ShouldHandleKeyedServices`.
- `MicrosoftExtensionsDI/TypeEvaluatorTests.cs`: `ShouldNotRegisterUnNeededBehaviors`.
- `MicrosoftExtensionsDI/CustomMediatorTests.cs`: `Can_Call_AddMediatr_multiple_times`.
- `MicrosoftExtensionsDI/NotificationPublisherTests.cs`: `ShouldResolveDefaultPublisher`, `ShouldSubstitutePublisherServiceType`.
- `MicrosoftExtensionsDI/PipelineTests.cs`: `Should_wrap_with_behavior` (doppelt zu `PipelineTests.cs`), `Should_handle_open_behavior_registration`, `Should_handle_inferred_behavior_registration`, `Should_handle_inferred_stream_behavior_registration`, `Should_handle_inferred_pre_processor_registration`, `Should_handle_inferred_post_processor_registration`, `Should_handle_open_behaviors_registration_from_a_single_type`, `Should_register_open_behaviors_correctly`.
- `MicrosoftExtensionsDI/StreamPipelineTests.cs`: `Should_wrap_with_behavior`, `Should_register_and_wrap_with_behavior` (doppelt zu `StreamPipelineTests.cs`).
