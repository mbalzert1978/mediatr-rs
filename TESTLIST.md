# Testliste

Siehe Skill `canon-tdd`.

## Request/Handler — `send`

Quelle: MediatR `test/MediatR.Tests/SendTests.cs`, `SendVoidInterfaceTests.cs`, `ExceptionTests.cs`.

- [x] Request mit registriertem Handler → `send` liefert die Antwort des Handlers · `send_returns_handler_response`
- [x] Request ohne Antwort (Antworttyp `()`) → `send` führt den Handler aus · `send_runs_handler_for_unit_response`
- [ ] Zwei Request-Typen mit je eigenem Handler → jeder Request erreicht nur seinen Handler
- [x] Request ohne registrierten Handler → `send` liefert `Err`, kein Panic · `send_without_handler_returns_err`
- [ ] Handler liefert einen Fehler → `send` gibt genau diesen Fehler zurück
- [ ] Zweimal `register` für denselben Request-Typ → der zuletzt registrierte Handler antwortet
- [ ] Fehler von `send` → lässt sich mit `?` als `Box<dyn std::error::Error>` weitergeben
