# EXTELIO – Betriebsdokumentation

## Inhalt

1. Architektur im Überblick
2. Netzwerk und Reverse Proxy
3. Startreihenfolge und Dienste
4. Konfigurationsgenerationen
5. Sicherung und Wiederherstellung
6. Telefonie-Core aktivieren
7. Runbooks
8. Bekannte Abweichungen von der Spezifikation

---

## 1. Architektur im Überblick

EXTELIO läuft als eine einzige Home-Assistant-App mit `host_network: true`.
Das ist keine Bequemlichkeit, sondern eine Notwendigkeit: SIP-Signalisierung
und RTP-Medienströme brauchen direkten Zugang zum Hostnetz, und die
RTP-Portspanne von 16384 bis 32768 lässt sich nicht sinnvoll durch eine
Bridge schleusen.

Die App besteht aus vier Prozessen:

| Dienst | Aufgabe |
|---|---|
| `pbx-bootstrap` | Läuft einmal beim Start: Verzeichnisse, Optionen, Migrationen, Schlüsselverwaltung. |
| `secret-broker` | Gibt Secrets nur an berechtigte Prozesse heraus und protokolliert jeden Zugriff. |
| `pbx-web` | Weboberfläche und API. |
| `pbx-worker` | Systemstatus, Home-Assistant-Aktualisierung, Aufbewahrungsfristen, Verbindung zum Telefonie-Core. |

Die fachliche Wahrheit liegt in der Datenbank. FreeSWITCH-Konfiguration wird
daraus erzeugt und nie von Hand bearbeitet.

---

## 2. Netzwerk und Reverse Proxy

Ein Reverse Proxy kann ausschließlich die Weboberfläche bedienen. SIP, SIPS
und RTP laufen daran vorbei, direkt auf den Host.

### Nginx Proxy Manager

Weil EXTELIO im Hostnetz läuft, gibt es keinen Add-on-Hostnamen wie
`local-extelio`. Der Proxy muss auf die IP-Adresse des Home-Assistant-Hosts
zeigen.

**Proxy Host anlegen:**

| Feld | Wert |
|---|---|
| Domain Names | der gewählte Hostname, z. B. `pbx.example.de` |
| Scheme | `http` |
| Forward Hostname / IP | die IP des HA-Hosts, z. B. `10.10.20.5` |
| Forward Port | `8099` |
| Websockets Support | eingeschaltet |
| Block Common Exploits | eingeschaltet |

Im Reiter **SSL** ein Zertifikat auswählen und **Force SSL** sowie **HTTP/2**
aktivieren.

**Add-on-Konfiguration passend dazu:**

```yaml
web_mode: http_behind_proxy
web_http_port: 8099
canonical_hostname: pbx.example.de
trusted_proxies:
  - 10.10.20.5
```

Hinweise aus der Praxis:

- Im Feld **Advanced** wirken `proxy_set_header`-Direktiven nicht wie erwartet,
  weil sie außerhalb des Location-Blocks landen. Die Standardfelder von NPM
  setzen die nötigen Weiterleitungs-Header bereits.
- Markdown-Codeblöcke, die versehentlich in das Advanced-Feld kopiert werden,
  führen beim Speichern zu einem internen Fehler.
- Damit der Hostname auch im lokalen Netz auflöst, im DNS-Filter (etwa AdGuard)
  einen Eintrag auf die IP des Proxys anlegen. Passkeys und Zertifikate binden
  an genau einen Namen; ein Wechsel zwischen IP und Hostname macht bereits
  registrierte Passkeys unbrauchbar.

### Ports

| Zweck | Voreinstellung | Anmerkung |
|---|---|---|
| Weboberfläche HTTP | 8099 | 8080 ist auf vielen Systemen belegt |
| Weboberfläche HTTPS | 8449 | nur bei `https_only` oder `http_https` |
| SIP LOCAL | 5060 / 5061 | interne Endgeräte |
| SIP PUBLIC/PUSH | 5080 / 5081 | standardmäßig aus |
| SIP TRUNK | 5090 / 5091 | Anbieterverbindungen |
| RTP | 16384–32768 | Medienströme |

Portkonflikte werden beim Start erkannt und mit Klartextmeldung abgewiesen.

### Externe Nebenstellen

Für Telefone außerhalb des Heimnetzes ist ein VPN der sichere Weg. Ein
Cloudflare-Tunnel trägt kein SIP; Tailscale hingegen funktioniert für diesen
Zweck gut. Der öffentliche SIP-Zugang bleibt bewusst standardmäßig aus.

---

## 3. Startreihenfolge und Dienste

```text
pbx-bootstrap → secret-broker → pbx-web / pbx-worker → FreeSWITCH → Abgleich
```

Schlägt `pbx-bootstrap` fehl, startet kein weiterer Dienst. Das ist Absicht:
eine Anlage mit unklarer Datenbank oder fehlender Schlüsselverwaltung soll
nicht halb hochkommen.

Jeder Dienst schreibt regelmäßig eine Lebenszeichen-Datei nach
`/data/state/`. Der Systemstatus wertet deren Alter aus: bis 90 Sekunden gilt
ein Dienst als gesund, bis 300 Sekunden als eingeschränkt, danach als gestört.

---

## 4. Konfigurationsgenerationen

Änderungen an Nebenstellen, Nummern, Trunks oder Flows wirken nicht sofort auf
die Telefonie. Der Weg ist bewusst zweistufig:

1. **Erzeugen** – aus dem gewünschten Zustand wird eine vollständige
   Konfiguration erzeugt und unter `/data/config/generation-N` abgelegt,
   zusammen mit einer Prüfsummenliste.
2. **Aktivieren** – ein Symlink `current` wird atomar umgehängt, danach lädt
   der Telefonie-Core neu.

Passt etwas nicht, stellt **Zurücksetzen** die vorherige Generation wieder her.
Weicht eine Datei von ihrer Prüfsumme ab, verweigert die Aktivierung den
Dienst und meldet die Abweichung.

Passwörter stehen in diesen Dateien nur als Referenz `{{secret:...}}`. Der
Klartext wird erst zur Laufzeit über den Secret Broker aufgelöst und landet
nie auf der Festplatte.

---

## 5. Sicherung und Wiederherstellung

Es gibt zwei Wege, die sich ergänzen:

- **Home-Assistant-Backup** – sichert die gesamte App im Rahmen der
  HA-Sicherung.
- **EXTELIO-Sicherung** – erzeugt ein eigenständiges, verschlüsseltes Archiv
  unter `/data/backup`.

Die EXTELIO-Sicherung läuft anwendungskonsistent: Während des Vorgangs werden
keine neuen Generationen aktiviert, die Datenbank wird sauber abgeschlossen
und als Momentaufnahme kopiert. Laufende Gespräche werden nicht unterbrochen.

Vor dem Einspielen prüft EXTELIO Prüfsumme und Schemaversion. Ein Archiv aus
einer neueren Version wird abgewiesen, statt eine unpassende Datenbank
einzuspielen.

**Wichtig:** Das Archiv ist mit dem Wurzelschlüssel dieser Installation
verschlüsselt. Geht `/data/kms/root.key` verloren, ist auch die Sicherung nicht
mehr lesbar. Beides gehört an getrennte Orte.

---

## 6. Telefonie-Core aktivieren

In der Voreinstellung wird das Image ohne FreeSWITCH gebaut. Alles außer der
eigentlichen Gesprächsvermittlung funktioniert dann bereits; der Systemstatus
weist den Telefonie-Core als *Wartung* aus.

Zum Einbinden:

```bash
WITH_FREESWITCH=1 ./scripts/build.sh amd64
```

Gebaut werden ausschließlich die Module aus `freeswitch/modules.conf`.
Skript-Engines und Fernsteuerungsschnittstellen wie `mod_xml_rpc`,
`mod_verto` oder `mod_lua` sind ausdrücklich ausgeschlossen.

Der Übersetzungsvorgang dauert je nach Maschine 20 bis 45 Minuten. Deshalb ist
der Weg über eine Build-Pipeline mit anschließendem Image-Abruf der
angenehmere.

---

## 7. Runbooks

### 7.1 Anbieter oder Trunk nicht erreichbar

1. **Systemstatus** prüfen: Steht `sofia-trunk` auf gestört?
2. **Trunks** öffnen und den Registrierungszustand ansehen.
3. Erreichbarkeit des Anbieters vom Host aus prüfen.
4. Zugangsdaten erneut hinterlegen. Das verlangt eine Bestätigung des eigenen
   Passworts und wird protokolliert.
5. Hilft nichts davon: **Protokolle** nach `trunk.` filtern und den Zeitpunkt
   der letzten erfolgreichen Registrierung suchen.

### 7.2 Zertifikat abgelaufen oder Erneuerung fehlgeschlagen

1. Der Systemstatus warnt ab 30 Tagen Restlaufzeit, ab 7 Tagen kritisch.
2. Prüfen, ob `canonical_hostname` gesetzt ist und der Name öffentlich auflöst.
3. Die Erneuerung braucht einen erreichbaren Weg zur Prüfung. Hinter einem
   Reverse Proxy übernimmt dieser üblicherweise das Zertifikat; dann ist
   `web_mode: http_behind_proxy` die richtige Wahl.
4. Bis zur Klärung bleibt die Anlage über den Proxy erreichbar. Passkeys
   funktionieren nur mit gültigem Zertifikat.

### 7.3 Angriffsversuche auf SIP

1. **Protokolle** nach `auth.` filtern.
2. Häufungen aus einem Netzbereich fallen über die maskierte Herkunft auf.
3. Der öffentliche SIP-Zugang sollte ausgeschaltet bleiben, solange keine
   externen Nebenstellen gebraucht werden.
4. Für externe Telefone ist ein VPN der bessere Weg als ein offener Port.
5. Die Sperren greifen automatisch und verlängern sich mit jedem Fehlversuch
   bis auf eine Stunde.

### 7.4 Datenbank meldet einen Integritätsfehler

1. Der Systemstatus zeigt `sqlite` als gestört.
2. Keine Änderungen mehr vornehmen.
3. Die App über Home Assistant stoppen.
4. Die letzte Sicherung prüfen und einspielen.
5. Anschließend Konfiguration neu erzeugen und aktivieren.

Ein Integritätsfehler lässt sich nicht übersteuern. Eine unlesbare Datenbank
ist kein Zustand, den man bewusst weiterbetreiben kann.

### 7.5 Schlüsselverwaltung nicht verfügbar

1. Fehlt `/data/kms/root.key`, startet die App nicht.
2. Ohne diese Datei sind alle gespeicherten Secrets endgültig unlesbar – das
   ist der beabsichtigte Schutz, kein Fehler.
3. Wiederherstellung nur aus einer Sicherung, die zum selben Schlüssel gehört.
4. Steht kein TPM zur Verfügung, arbeitet EXTELIO mit einem zufällig erzeugten
   Schlüssel auf der Festplatte. Einen softwareseitigen Ersatz für
   Hardwarebindung gibt es bewusst nicht, weil er nur Sicherheit vortäuschen
   würde.

### 7.6 Konfigurationsgeneration beschädigt

1. Die Aktivierung meldet eine Abweichung vom Prüfsummenverzeichnis.
2. Über **Sicherheit → Zurücksetzen** auf die vorherige Generation gehen.
3. Anschließend neu erzeugen; dabei entsteht eine frische Generation aus dem
   gespeicherten Zustand.
4. Die beschädigte Generation bleibt zur Nachschau liegen.

---

## 8. Bekannte Abweichungen von der Spezifikation

Diese Ausgabe setzt die Spezifikation weitgehend um. Die folgenden Punkte sind
bewusst offen und werden hier benannt, statt sie zu verschweigen:

| Punkt | Stand | Begründung |
|---|---|---|
| Passkey / WebAuthn | offen | Setzt einen sicheren Kontext mit festem Hostnamen voraus. Der verpflichtende Rückfallweg aus Passwort und zweitem Faktor ist vollständig umgesetzt. |
| Telefonie-Core | über Build-Parameter | Der Quellcode-Build von FreeSWITCH ist vorbereitet, aber nicht voreingestellt, damit die App zuverlässig installierbar bleibt. |
| Hardwaregebundener Wurzelschlüssel | erkannt, nicht genutzt | Die Erkennung ist vorhanden; die TPM-Bindung selbst folgt. |
| Gesprächsaufzeichnung | offen | Setzt den laufenden Telefonie-Core voraus. |
| Automatische Geräteeinrichtung | Grundgerüst | Freigabezustand und Datenmodell stehen; die Auslieferung der Gerätekonfigurationen folgt. |
| Communication Map | offen | Die Topologieansicht folgt, sobald Live-Daten vorliegen. |
| Signatur der Artefakte | in der Pipeline | Braucht die Veröffentlichungsidentität des Betreibers. |

Die vollständige Zuordnung von Anforderung, Umsetzung und Nachweis steht in
`compliance/requirements.yaml`.
