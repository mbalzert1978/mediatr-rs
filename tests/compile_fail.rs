//! Tests für die Punkte „… kompiliert nicht“ in den Abschnitten „Pipeline — Behaviors“, „Pipeline — Vor- und Nachverarbeiter“, „Pipeline — Fehler-Handler und Fehler-Aktionen“ und „Stream-Request — Stream-Behaviors“ in `TESTLIST.md`.

// trybuild prüft selbst: Der Test scheitert, wenn die Datei kompiliert oder eine andere Fehlermeldung liefert.
#[test]
fn behavior_registered_after_first_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/behavior_after_handler.rs");
}

#[test]
fn behavior_registered_after_first_notification_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/behavior_after_subscribe.rs");
}

#[test]
fn behavior_requiring_clone_rejects_non_clone_request_at_compile_time() {
    trybuild::TestCases::new().compile_fail("tests/ui/behavior_requires_clone.rs");
}

#[test]
fn post_processor_rejects_non_clone_request_at_compile_time() {
    trybuild::TestCases::new().compile_fail("tests/ui/post_processor_requires_clone.rs");
}

#[test]
fn pre_processor_registered_after_first_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/pre_processor_after_handler.rs");
}

#[test]
fn post_processor_registered_after_first_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/post_processor_after_handler.rs");
}

#[test]
fn error_handler_registered_after_first_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/error_handler_after_handler.rs");
}

#[test]
fn error_action_registered_after_first_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/error_action_after_handler.rs");
}

#[test]
fn error_handler_rejects_non_clone_request_at_compile_time() {
    trybuild::TestCases::new().compile_fail("tests/ui/error_handler_requires_clone.rs");
}

#[test]
fn error_action_rejects_non_clone_request_at_compile_time() {
    trybuild::TestCases::new().compile_fail("tests/ui/error_action_requires_clone.rs");
}

#[test]
fn stream_behavior_registered_after_first_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/stream_behavior_after_handler.rs");
}
