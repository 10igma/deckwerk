# Auftrag: DJ-Software nach Spezifikation bauen

## Ausgangslage

Im Repository liegt unter `docs/spezifikation.md` die vollständige Spezifikation einer Desktop-DJ-Software für Techno. Baue das Produkt danach, Ausbaustufe für Ausbaustufe, bis alle Muss- und Soll-Anforderungen erfüllt sind. Die Spezifikation ist verbindlich; wo sie eine Lücke oder einen Widerspruch hat, triff eine begründete Entscheidung, halte sie fest und arbeite weiter.

## Arbeitsweise

- Arbeite die Ausbaustufen aus dem Umsetzungsplan der Spezifikation der Reihe nach ab. Jede Stufe endet mit einer startbaren App, grünen Tests und einem gepushten Commit.
- Führe `docs/fortschritt.md`: erledigte Anforderungen (mit Kennung), offene Punkte, getroffene Entscheidungen, nächster Schritt. Die Arbeit läuft über viele Sessions; eine neue Session muss allein aus Repository und dieser Datei weiterarbeiten können. Lies sie deshalb zu Beginn jeder Session zuerst.
- Richte früh eine automatische Build-Pipeline ein, die bei jedem Push installierbare Pakete für Windows, macOS und Linux erzeugt.
- Halte nur an, wenn etwas ohne mich wirklich nicht weitergeht. Fragen, die die laufende Stufe nicht blockieren, sammelst du in `docs/fortschritt.md`.

## Design ohne Figma

Das Design entsteht direkt im Code und wird an der laufenden App geprüft.

1. Lege vor dem ersten UI-Code ein Designsystem fest: Farben, Typografie, Abstände, Zustände und Grundkomponenten als zentrale Design-Tokens. Dunkle Oberfläche, im schwach beleuchteten Raum gut lesbar, eigenständig und nicht an bestehende DJ-Software angelehnt.
2. Setze dafür zwei unabhängige Subagenten ein:
   - **Art Director:** visuelle Richtung, Hierarchie, Farbe, Eigenständigkeit, Konsistenz.
   - **UX-Designer:** Bedienabläufe, Tastaturbedienung, Lesbarkeit unter Zeitdruck, Fehlervermeidung beim Live-Mixen.
3. Nach jeder Stufe mit sichtbarer Oberfläche: App starten, jede Ansicht in ihren wichtigen Zuständen als Screenshot aufnehmen (leer, Track geladen, laufend, Übergang, Fehler) und beiden Rollen zur Prüfung gegen Designsystem und Spezifikation geben. Gib ihnen die Screenshots und die Anforderungen, nicht deine eigene Einschätzung. Behebe die Mängel und wiederhole die Prüfung, bis keine wesentlichen mehr offen sind.

## Prüfung der Audio-Funktionen

Du kannst nicht hören, also prüfe messbar:

- **Analyse:** Tests gegen die Referenztracks in `testdaten/` mit bekannten BPM- und Tonart-Werten; die Trefferquoten aus der Spezifikation sind das Abnahmekriterium.
- **Engine und Übergänge:** Offline rendern und das Ergebnis vermessen: Beats synchronisierter Decks liegen innerhalb der Toleranz übereinander, kein Clipping, keine Aussetzer, Lautheit im Zielbereich, Reglerverläufe entsprechen dem Plan.
- **Auto-Mix:** Für ein festes Test-Set einen Mix rendern und prüfen, dass jeder Übergang auf einer Phrasengrenze liegt und die Tonart-Regeln eingehalten sind.

Was sich nur am echten Gerät beurteilen lässt (Klangqualität, Latenz am Audio-Interface, Kopfhörer-Vorhören, Gefühl der Bedienung), schreibst du pro Stufe als kurze Abnahmeliste für mich auf, mit genauen Schritten zum Nachprüfen.

## Bericht am Ende jeder Stufe

Kurz und ehrlich: was gebaut wurde, was automatisch geprüft ist und mit welchem Ergebnis, was ungeprüft ist, was ich selbst testen soll, was als Nächstes kommt. Schlägt ein Test fehl oder fehlt etwas, steht das im Bericht.
