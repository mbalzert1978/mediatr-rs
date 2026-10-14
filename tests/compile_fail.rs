//! Tests für die Punkte „… kompiliert nicht“ in den Abschnitten „Pipeline — Behaviors“ und „Pipeline — Vor- und Nachverarbeiter“ in `TESTLIST.md`.

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
fn behavior_requiring_clone_makes_send_for_non_clone_request_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/behavior_requires_clone.rs");
}

#[test]
fn post_processor_makes_send_for_non_clone_request_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/post_processor_requires_clone.rs");
}

#[test]
fn pre_processor_registered_after_first_handler_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/pre_processor_after_handler.rs");
}
