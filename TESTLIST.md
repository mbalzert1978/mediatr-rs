# Testliste

Siehe Skill `canon-tdd`.

## Request/Handler — `send`

Quelle: MediatR `test/MediatR.Tests/SendTests.cs`, `SendVoidInterfaceTests.cs`, `ExceptionTests.cs`.

- [x] Request mit registriertem Handler → `send` liefert die Antwort des Handlers · `send_returns_handler_response`
- [ ] Request ohne Antwort (Antworttyp `()`) → `send` führt den Handler aus
- [ ] Zwei Request-Typen mit je eigenem Handler → jeder Request erreicht nur seinen Handler
- [ ] Request ohne registrierten Handler → `send` liefert `Err`, kein Panic
- [ ] Handler liefert einen Fehler → `send` gibt genau diesen Fehler zurück
