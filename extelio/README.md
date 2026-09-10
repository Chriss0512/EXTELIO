# EXTELIO

Telefonanlage für Home Assistant. Eigene Weboberfläche, eigene Anmeldung,
nachvollziehbare Konfiguration.

## Installation

1. Add-on installieren.
2. Unter **Konfiguration** die Ports prüfen. Die Voreinstellung nutzt 8099 für
   die Weboberfläche, weil 8080 auf vielen Systemen schon belegt ist.
3. Add-on starten.
4. Die Weboberfläche unter `http://<ip-des-hosts>:8099` öffnen.
5. Beim ersten Aufruf den Systemadministrator anlegen und die
   Zwei-Faktor-Authentifizierung einrichten.

Es gibt keine voreingestellten Zugangsdaten. Ohne abgeschlossene
Erstinbetriebnahme ist kein Zugriff möglich.

## Erste Schritte nach der Anmeldung

1. **Trunks** – die Verbindung zum Telefonanbieter anlegen.
2. **Rufnummern** – die Nummern des Anschlusses eintragen; sie werden nach
   E.164 normalisiert.
3. **Nebenstellen** – interne Nummern anlegen. Dabei entsteht automatisch ein
   starkes SIP-Passwort, das genau einmal angezeigt wird.
4. **Geräte** – Telefone registrieren und einer Leitung zuordnen.
5. **Flows** – festlegen, was mit eingehenden Anrufen passiert, und den Flow
   prüfen lassen.
6. **Sicherheit** – Konfiguration erzeugen und aktivieren.

## Wichtige Einstellungen

| Option | Bedeutung |
|---|---|
| `web_mode` | Betriebsart der Weboberfläche. Hinter einem Reverse Proxy `http_behind_proxy` wählen. |
| `canonical_hostname` | Fester Hostname. Wird für Zertifikate und Passkeys benötigt. |
| `trusted_proxies` | Quellen, deren Weiterleitungs-Header ausgewertet werden dürfen. |
| `public_push_enabled` | Öffentlicher SIP-Zugang. Standardmäßig aus. |
| `local_sip_port` und folgende | Ports der drei SIP-Profile. Konflikte werden beim Start gemeldet. |

## Sicherheitsversprechen

- Kein Standardpasswort, kein offener Zugang.
- Passwörter mit Argon2id, zweiter Faktor verpflichtend.
- Secrets ausschließlich verschlüsselt, niemals in Umgebungsvariablen.
- Kein Klartext-Passwort in erzeugten Konfigurationsdateien.
- An Home Assistant gehen nur technische Zustände, keine personenbezogenen Daten.

Ausführliche Betriebshinweise und Notfallabläufe stehen in `DOCS.md`.
