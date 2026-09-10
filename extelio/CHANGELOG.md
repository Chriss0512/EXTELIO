# Änderungen

Die Versionierung folgt [Semantic Versioning](https://semver.org/lang/de/).

## [0.1.0] – 2026-09-10

Erste Ausgabe. Aufbau der Anwendung nach der Implementierungsspezifikation.

### Enthalten

**Anmeldung und Rechte**
- Erstinbetriebnahme mit Anlage des ersten Administrators, ohne
  voreingestellte Zugangsdaten
- Passwörter mit Argon2id und individuellem Salt
- Zweiter Faktor nach RFC 6238, gegen die Referenzvektoren geprüft
- Serverseitige Sitzungen mit getrennten Cookies für HTTP und HTTPS,
  Leerlauf- und Höchstdauer, Erneuerung nach Rechteänderung
- Fünf Rollenvorlagen mit abgestuften Berechtigungen
- Erneute Bestätigung des Passworts vor kritischen Aktionen
- Sperren nach Konto, Adresse, Netzbereich und Verfahren, mit wachsender Dauer

**Schlüssel und Secrets**
- Verschlüsselung in zwei Stufen mit AES-256-GCM
- Neun Secret-Klassen mit eigenen Regeln für Einsicht, Wechsel und Zugriff
- Secret Broker als eigener Dienst; keine Verteilung über Umgebungsvariablen
- Schlüsselwechsel und kryptografische Löschung

**Telefonie**
- Nebenstellen, Geräte, Leitungen, Rufnummern, Anbieterprofile, Trunks,
  Rufgruppen
- Rufnummern werden nach E.164 normalisiert
- Routing als typisierter Graph mit Prüfung auf Referenzen, Sackgassen und
  Kreisläufe sowie mit Anrufsimulation
- Erzeugung der FreeSWITCH-Konfiguration aus dem gespeicherten Zustand,
  strukturell und ohne Textverkettung
- Drei SIP-Profile mit getrennten Zugriffslisten; öffentlicher Zugang aus

**Betrieb**
- Konfigurationsstände mit Prüfsummen, atomarer Aktivierung und Rücksprung
- Erkennung nachträglicher Änderungen an erzeugten Dateien
- Systemstatus mit allen Pflichtprüfungen, Schwellwerten für Speicher und
  Zertifikate
- Sicherung im laufenden Betrieb, verschlüsselt, mit Prüfung vor dem
  Einspielen
- Zweckgebundene Aufbewahrungsfristen je Datenart
- Sicherheitsprotokoll als verkettetes Journal mit regelmäßigen Prüfmarken

**Oberfläche**
- Eigene Weboberfläche ohne Inline-Skripte und Inline-Styles
- Designsystem, Iconfamilie und Komponentensatz nach Spezifikation
- Flow-Ansicht mit Diagramm, Prüfung und Simulation

**Home Assistant**
- Fünf Sensoren mit technischen Zuständen
- Personenbezogene Daten werden technisch daran gehindert, die Anlage zu
  verlassen

### Bekannte Einschränkungen

- Passkey und WebAuthn folgen mit der Zertifikatseinrichtung
- Der Telefonie-Core wird über den Build-Parameter `WITH_FREESWITCH`
  eingebunden und ist nicht voreingestellt
- Gesprächsaufzeichnung, automatische Geräteeinrichtung und Topologieansicht
  sind vorbereitet, aber noch nicht nutzbar

Einzelheiten stehen in `DOCS.md`, Abschnitt 8.
