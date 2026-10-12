# MediatR-Kernkonzepte

Was der Port an Verhalten abbilden muss. Gelesen aus `src/MediatR` und `src/MediatR.Contracts` von
<https://github.com/LuckyPennySoftware/MediatR>. Lies das, bevor du die Testliste für ein neues Konzept schreibst.
Die Rust-Form entsteht erst im TDD-Zyklus, nicht hier.

| Konzept (C#) | Verhalten in MediatR |
| --- | --- |
| `IRequest<TResponse>` + `IRequestHandler` | `Send` schickt einen Request an **genau einen** Handler und liefert dessen Antwort. Fehlt der Handler, schlägt `Send` fehl. |
| `IRequest` (ohne Antwort) | Request ohne Rückgabewert. |
| `INotification` + `INotificationHandler` | `Publish` schickt an **null bis viele** Handler. Null Handler ist kein Fehler. Derselbe Handler-Typ läuft nur einmal, auch wenn doppelt registriert. |
| `INotificationPublisher` (`ForeachAwait`, `TaskWhenAll`) | Handler laufen nacheinander (Standard; erster Fehler stoppt) oder alle gleichzeitig. |
| `IPipelineBehavior` | Umhüllt den Handler; ruft `next` oder bricht ab. Zuerst registriert = ganz außen. |
| `IRequestPreProcessor` / `IRequestPostProcessor` | Läuft vor bzw. nach dem Handler; in MediatR selbst als Behavior gebaut. |
| `IRequestExceptionHandler` / `IRequestExceptionAction` | Handler darf bei einem Fehler eine Ersatz-Antwort setzen; Action macht nur Seiteneffekte, der Fehler läuft weiter. |
| `IStreamRequest` + Handler + Stream-Behavior | Wie Request, die Antwort ist ein asynchroner Strom. |
| `ISender` / `IPublisher` / `IMediator` | Send/Stream, Publish, beides. |

Bewusst nicht portiert: Lizenzprüfung, Assembly-Scan, `Send(object)`/`Publish(object)`, Service-Lifetimes.
