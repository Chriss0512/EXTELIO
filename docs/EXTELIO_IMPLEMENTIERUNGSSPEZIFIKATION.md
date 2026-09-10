# EXTELIO – Implementierungs- und Rebuild-Spezifikation

**Produkt:** EXTELIO  
**Claim:** Software-defined Communications  
**Produktart:** Software-defined PBX / Business Communications Platform  
**Dokumentstatus:** ARCHITECTURE\_COMPLETE / IMPLEMENTATION\_LOCK\_PENDING  
**Stand:** 2026-09-10  
**Zweck:** Kanonische Minimal-Spezifikation des final gewählten Systemzustands. Historische Alternativen, verworfene Varianten und Entscheidungsdiskussionen sind nicht Bestandteil dieses Dokuments.

> \*\*Leitprinzip:\*\* Simple outside, rigorous inside.

\---

# 1\. Produktidentität und Ziel

EXTELIO ist eine **einzige direkt installierbare Home-Assistant-App** für Telefonie, PBX, Routing, Provisioning und Kommunikationsverwaltung.

Verbindlich:

* genau eine installierte HA-App;
* kein separater PBX-Host, keine VM, kein Docker Compose;
* kein Home-Assistant-Ingress;
* eigenes Web-Frontend und eigene Authentifizierung;
* FreeSWITCH als Telephony Core;
* lokale Nutzung ohne verpflichtende Herstellercloud;
* Groundwire/Acrobits-Push optional;
* Wizard-first, Progressive Disclosure, Expert Mode;
* Domain Model ist die fachliche Source of Truth;
* FreeSWITCH-Konfiguration wird ausschließlich aus dem Desired State erzeugt;
* Standards werden als testbare Anforderungen mit Evidence umgesetzt.

## 1.1 Markenbotschaften

Primär:

```text
EXTELIO
Software-defined Communications
```

Kommunikationsbotschaft:

```text
Verbinden.
Steuern.
Einfacher.

Kommunikation als Plattform.
```

Markenattribute:

```text
FLEXIBLE
SECURE
SCALABLE
INDEPENDENT
```

## 1.2 Produktfamilie

Reservierte Modulnamen:

|Name|Funktion|
|-|-|
|EXTELIO Core|PBX / Call Control|
|EXTELIO Flow|Routing und Call Flows|
|EXTELIO Desk|Web-/Desktop-Client|
|EXTELIO Mobile|Smartphone-Client|
|EXTELIO Edge|Standort-/Gateway-Komponente|
|EXTELIO Connect|Provider und externe Integrationen|
|EXTELIO Provision|Endgeräte- und Provisionierungsverwaltung|

Die Home-Assistant-App wird als **EXTELIO** angezeigt; `EXTELIO Core` bezeichnet intern den PBX-/Call-Control-Kern.

\---

# 2\. Designsystem

## 2.1 Design-DNA

EXTELIO soll wie eine moderne Infrastruktur- und Kommunikationsplattform wirken, nicht wie eine klassische Telefonanlage.

Gestalterische Gewichtung:

```text
70 % UniFi:
Informationsarchitektur, Ruhe, Flächen, Navigation,
Status, Geräteverwaltung, Listen-/Detailprinzip

30 % 3CX:
Dialer, Presence, Teams, Extensions,
Telefonie- und Kommunikationsworkflows
```

Keine Oberfläche oder Marke wird kopiert.

Visuelle Eigenschaften:

* präzise;
* modular;
* vernetzt;
* souverän;
* technisch;
* ruhig;
* skalierbar;
* professionell.

## 2.2 Markenregel

> EXTELIO visualisiert Kommunikation als Signalfluss, Routing und Verbindung – nicht als Telefonhörer.

Zu vermeiden:

* Telefonhörer als primäres Markenzeichen;
* klassische Sprechblase als Logo;
* Funkwellen-über-Hörer-Symbolik;
* Globus-/Weltkugellogo;
* generische Punktnetzwerk-Logos;
* Neon-/Cyberpunk-Ästhetik;
* dekorative SaaS-Kacheln ohne funktionale Bedeutung.

## 2.3 Logo und Signet

Logoformen:

```text
Primary    = Signet + EXTELIO + Claim
Compact    = Signet + EXTELIO
Icon       = Signet allein
Monochrome = Schwarz / Weiß
```

Das Signet basiert auf einem abstrahierten **X** aus Signalpfaden / Routing / Cross-Connect / Exchange.

Es muss funktionieren als:

* App Icon;
* Favicon;
* Browser Tab Icon;
* Installer Icon;
* Mobile Icon;
* System Tray Icon.

## 2.4 Signal Paths

Zentrales grafisches Leitmotiv:

* dünne geometrische Linien;
* verbinden, verzweigen, kreuzen und routen;
* enden an Nodes;
* Zustandswechsel dürfen sichtbar werden;
* reduziert, nicht wie eine Leiterplatte.

Einsatz:

* Login;
* Dashboard;
* Flow Editor;
* Communication Map;
* Website;
* Dokumentation;
* Präsentationen.

## 2.5 Farbpalette

|Token|Hex|Verwendung|
|-|-|-|
|Extelio Blue|`#2864DC`|Primary, CTA, aktive Navigation, Links|
|Extelio Blue Bright|`#3B82F6`|Hover, Focus, aktive technische Zustände|
|Extelio Ink|`#111827`|Haupttext, Dark Background|
|Extelio Graphite|`#232934`|Dark Surfaces|
|Extelio Slate|`#667085`|Sekundärtext, Metadaten|
|Extelio Border|`#E4E7EC`|Border, Divider, Tabellen|
|Extelio Canvas|`#F6F7F9`|Light Background|
|Extelio Surface|`#FFFFFF`|Panels, Karten, Tabellen|
|Success / Online|`#22C55E`|funktionaler Status|
|Warning / Degraded|`#F59E0B`|funktionaler Status|
|Critical / Offline|`#EF4444`|funktionaler Status|
|Info|`#3B82F6`|funktionaler Status|

Regel: ca. 85–90 % der UI bestehen aus neutralen Flächen/Textfarben; Blau ist gezielter Interaktions-/Markenakzent.

## 2.6 Light / Dark

**Light Mode ist die primäre Designreferenz.**

Light:

```text
Background       #F6F7F9
Surface          #FFFFFF
Primary Text     #111827
Secondary Text   #667085
Border           #E4E7EC
Primary Accent   #2864DC
```

Dark:

```text
Background        #111827
Surface           #232934
Secondary Surface #2D3440
Primary Text      #F9FAFB
Secondary Text    #98A2B3
Border            #374151
Primary Accent    #3B82F6
```

Dark Mode ist funktional gleichwertig und wird aus denselben semantischen Tokens abgeleitet.

## 2.7 Typografie

Bevorzugt: **Inter**  
Alternative: **Geist**

|Verwendung|Größe|Gewicht|
|-|-:|-:|
|Page Title|24–28 px|600|
|Section Title|18–20 px|600|
|Card/Panel Title|15–16 px|600|
|Body|14 px|400|
|Table|13–14 px|400–500|
|Metadata|12 px|400|
|Button|13–14 px|500–600|

Wortmarke:

* EXTELIO in Versalien;
* SemiBold/Bold;
* leicht erhöhtes Letterspacing;
* breit, ruhig, technisch;
* keine futuristische Verzerrung.

## 2.8 Geometrie

```text
Spacing Grid   4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 px
Radius         8 px Standard
Large Panels   10–12 px
Buttons        7–8 px
Status Pills   999 px / fully rounded
Border         1 px
```

Schatten nur minimal; Trennung bevorzugt über Border und Fläche.

## 2.9 Buttons und Inputs

Buttons:

```text
Primary:
background #2864DC
text white
height 36–40
radius 8
weight 600
hover #3B82F6

Secondary:
white/transparent
border #D0D5DD
text #344054

Ghost:
transparent
neutral
subtle hover

Destructive:
red only for destructive operations

Icon Button:
32–36 px
tooltip mandatory
```

Inputs:

```text
height 38–40
radius 8
border #D0D5DD
focus Extelio Blue
label above
help text below
```

Advanced Settings standardmäßig eingeklappt.

## 2.10 Grundlayout

Permanente linke Navigation + reduzierte Topbar.

```text
┌────────────────────────────────────────────────────────────┐
│ EXTELIO                       Search        Status / User   │
├───────────────┬────────────────────────────────────────────┤
│ Navigation    │ Main Workspace                             │
│               │                                            │
└───────────────┴────────────────────────────────────────────┘
```

Navigation:

```text
COMMUNICATE
- People
- Dialer
- Messages
- Calls
- Voicemail

MANAGE
- Users
- Extensions
- Devices
- Numbers
- Routing
- Groups
- Trunks
- Flows
- Provisioning

SYSTEM
- Overview
- Network
- Security
- Updates
- Logs
- Settings
```

Sichtbarkeit ist rollen-/rechtebasiert.

## 2.11 Zentrales UX-Prinzip

```text
Liste -> Auswahl -> Detailpanel
```

Objekte werden möglichst ohne Kontextverlust in einem rechten Detailpanel bearbeitet.

Tabellen:

* sortierbar;
* filterbar;
* Bulk Actions;
* Row Selection;
* Kontextmenü `…`;
* Klick öffnet Detailpanel;
* Density: `Comfortable` / `Compact`.

## 2.12 Statusdarstellung

Farbe allein reicht nie.

Beispiele:

```text
● ONLINE
Registered · 12 ms

● DEGRADED
Packet loss · 3.4 %

● OFFLINE
Last seen · 14:32
```

## 2.13 Dashboard

Fokus: **Systemzustand und Kommunikationsgesundheit**, nicht dekorative KPIs.

Primäre Bereiche:

```text
Extensions
Devices
SIP Trunks
Active Calls
Call Activity
System Status
Recent Calls
Top Endpoints
```

Personenbezogene Inhalte folgen Privacy-/RBAC-Regeln.

## 2.14 EXTELIO Flow

EXTELIO Flow ist der visuelle Editor für den typisierten Routing Graph.

Unterstützte Nodes mindestens:

* Incoming Number;
* Schedule / Business Hours;
* User / Extension;
* Ring Group;
* Queue;
* IVR;
* Announcement;
* Voicemail;
* External Number;
* Condition;
* Fallback / End.

Simulation und Live-Diagnose dürfen aktive Signalwege visuell hervorheben.

## 2.15 Communication Map

Topologie-/Diagnoseansicht für:

* Core;
* Sites;
* SIP Provider;
* Trunks;
* Gateways;
* Edge Nodes;
* Devices;
* Mobile Clients;
* Softphones.

Zweck: Übersicht, Diagnose, Navigation.

## 2.16 Iconographie

* Outline;
* geometrisch;
* 1.5–2 px Stroke;
* klare Silhouetten;
* reduzierte Details.

Iconfamilie mindestens:

```text
User
Extension
Device
Number
Route
Trunk
Queue
Group
Gateway
Flow
Provisioning
Security
Network
Call
Voicemail
Settings
```

Telefonhörer ist als Funktionsicon zulässig, aber kein dominantes Markensymbol.

## 2.17 Verbindlicher UI-Komponentensatz

Mindestens umzusetzen:

```text
Primary Logo
Compact Logo
App Icon / Favicon
Color Tokens
Typography
Primary / Secondary / Ghost / Destructive Buttons
Inputs / Selects / Toggles / Checkboxes
Status Badges
Sidebar Navigation
Topbar
Tables
Detail Panel
Device Card
Dashboard Card
Call / Presence Elements
Dialer
Flow Nodes / Connections
Communication Map Nodes
Empty States
Loading States
Error States
Warning States
Tooltips
Dialogs / Confirmation
Notifications
```

Geräte-Detailansichten enthalten mindestens `Overview`, `Configuration`, `Lines`, `Network`, `Provisioning`, `Logs`.

## 2.18 Designartefakte

Repository muss enthalten:

```text
brand/
├── logo-primary.svg
├── logo-compact.svg
├── icon.svg
├── icon-512.png
├── favicon.svg
├── brandboard.png
└── tokens.json

src/web/src/design/
├── tokens.css
├── typography.css
├── icons/
└── components/
```

**Visuelle Referenz:** `EXTELIO\_BRANDBOARD.png`

\---

# 3\. Normative Basis

Verbindlich zu berücksichtigen und in einer Requirements-/Evidence-Matrix abzubilden:

* DSGVO / Privacy by Design / Privacy by Default;
* ISO/IEC 25010:2023;
* ISO/IEC 27001:2022 als organisatorische Referenz, keine ungeprüfte Zertifizierungsbehauptung;
* BSI IT-Grundschutz NET.4.1 TK-Anlagen;
* BSI IT-Grundschutz NET.4.2 VoIP;
* Common Criteria CC:2022 / CEM:2022;
* OWASP Top 10;
* OWASP ASVS 5.0.0: Level 2 vollständig + gezielte Level-3-Anforderungen;
* NIST SP 800-218 SSDF v1.1;
* RFC 3261 SIP;
* RFC 3550 RTP;
* RFC 3711 SRTP;
* RFC 4568 SDP Security Descriptions;
* RFC 5630 SIPS.

Compliance-Ziel: **certification-ready**, nicht „zertifiziert“.

Responsibilities:

```text
PRODUCT
OPERATOR
SHARED
NOT\_APPLICABLE + Begründung
```

\---

# 4\. Systemarchitektur

## 4.1 Deployment

```text
Home Assistant OS / Supervisor
└── EXTELIO App
    ├── pbx-web
    ├── pbx-core
    ├── pbx-worker
    ├── pbx-xml-adapter
    ├── secret-broker
    ├── FreeSWITCH
    └── Certbot job
```

Verbindlich:

```text
host\_network: true
Protection Mode: on
Ingress: false
Docker API: false
SYS\_ADMIN: false
NET\_ADMIN: false
unnötige Mounts/Devices: false
Custom AppArmor: required
```

Host Networking ist ein dokumentierter Trade-off und wird durch Prozess-, UID-, Dateirechte-, ACL- und Authentifizierungsgrenzen kompensiert.

## 4.2 Runtime und Komponenten

|Bereich|Festlegung|
|-|-|
|Base|Home Assistant Base Image, Multi-Stage Build|
|Backend|Rust + Axum|
|Frontend|React + TypeScript strict|
|Telephony|FreeSWITCH|
|Process Supervisor|s6-overlay aus HA Base|
|Database|SQLite + WAL|
|ACME|Certbot|
|Config|Desired State + Compiler|
|Media|FreeSWITCH Media Anchor|
|HA UI|eigene Weboberfläche, kein Ingress|

Unix-Principals mindestens:

```text
pbx-web
pbx-core
pbx-worker
freeswitch
certbot
```

\---

# 5\. Netzwerk, SIP und Ports

## 5.1 Sofia-Sicherheitsprofile

Drei getrennte Profile:

```text
LOCAL
PUBLIC/PUSH
TRUNK
```

Jedes besitzt eigene:

* Bindings;
* Authentifizierung;
* ACL/Policy;
* Dialplan Context;
* Rate Limits;
* Transportregeln.

## 5.2 Standard-Bindings

Single-IP-Standard:

|Funktion|Default|
|-|-|
|Web HTTP|`8080/TCP`|
|Web HTTPS|`8443/TCP`|
|LOCAL SIP|`5060 UDP/TCP`|
|LOCAL SIPS|`5061/TCP`|
|PUBLIC/PUSH SIP|`5080 UDP/TCP`|
|PUBLIC/PUSH SIPS|`5081/TCP`|
|TRUNK SIP|`5090 UDP/TCP`|
|TRUNK SIPS|`5091/TCP`|
|RTP/SRTP|`16384–32768/UDP`|
|ESL|`127.0.0.1:8021/TCP`|

Ports sind frei konfigurierbar, aber Konflikte werden validiert.

Advanced:

```text
separate Bind-IP je Sofia-Profil
-> 5060/5061 je IP möglich
```

## 5.3 Web-Transport

Unterstützt:

```text
HTTPS only
HTTP only
HTTP + HTTPS
HTTP origin behind trusted TLS reverse proxy/tunnel
```

HTTPS ist Ziel/default, HTTP bleibt bewusst zulässig.

Passkeys/WebAuthn nur in gültigem Secure Context.

## 5.4 SIP-/Media-Security

Default:

```text
LOCAL       STRICT
PUBLIC/PUSH STRICT
TRUNK       stärkstes providerkompatibles Profil
```

STRICT umfasst:

* TLS wo anwendbar;
* SRTP für PUBLIC/PUSH;
* keine Guest Calls;
* starke Credentials;
* Rate Limits;
* kein stiller Downgrade.

COMPATIBLE/LEGACY nur explizit.

## 5.5 Externe Erreichbarkeit

Default:

```text
Public SIP: OFF
DDNS automation: OFF
Public-IP automation: OFF
UPnP/NAT-PMP: OFF
```

Router-/Firewallregeln werden angezeigt und per Preflight geprüft, aber nicht automatisch verändert.

Split-horizon DNS ist empfohlen, aber nicht zwingend.

Voice VLAN ist empfohlen, aber nicht zwingend.

IPv6: unterstützt, Default `false`.

## 5.6 ESL

```text
listen-ip: 127.0.0.1
listen-port: 8021
apply-inbound-acl: loopback
password: random >=256-bit entropy
nat-map: false
```

Nie das bekannte FreeSWITCH-Standardpasswort.

\---

# 6\. Identity \& Access Management

## 6.1 Login

Bevorzugt:

```text
Passkey / WebAuthn
```

Fallback:

```text
Username + Password + TOTP
```

Jeder menschliche Nutzer muss ein lokales Passwort besitzen.

Biometrie wird ausschließlich lokal durch Passkey/WebAuthn verarbeitet; EXTELIO speichert keine biometrischen Rohdaten/Templates.

## 6.2 Password Security

```text
Argon2id
individual salt
hardware-backed pepper wenn verfügbar
kein Pepper wenn TPM/HSM nicht verfügbar
kein softwarebasierter Ersatz-Pepper
```

Parameter werden sicher kalibriert und versioniert.

## 6.3 MFA

TOTP:

```text
RFC 6238
6 digits
30 s
±1 window
```

WebAuthn über etablierte Safe-API-Library.

## 6.4 Sessions

* serverseitig in SQLite;
* opaque Cookie;
* keine Auth-Tokens im `localStorage`;
* widerrufbar;
* Idle-/Absolute Timeout;
* Session Rotation.

HTTPS-Cookie:

```text
Secure
HttpOnly
SameSite=Strict
Path=/
```

HTTP-Modus benutzt getrennte Session/Cookie-Namen und übernimmt keine HTTPS-Session.

## 6.5 RBAC

Templates:

```text
Systemadministrator
Telefonieadministrator
Supervisor
Benutzer
Nur Lesen
```

Zusätzlich benutzerdefinierte Rollen + granulare Permissions.

Scopes:

```text
Organization
-> Site
-> Department
-> Object/User/Queue/Number
```

## 6.6 Step-up

Kritische Aktionen verlangen erneute Passworteingabe; Passkey darf zusätzlich/alternativ bestätigen.

Besonders kritische Policies können Passwort + Passkey verlangen.

## 6.7 Recovery

E-Mail-basierter Recovery-Prozess:

* einmalige zufällige Tokens;
* serverseitig gehasht;
* kurze Gültigkeit;
* keine Account-Enumeration;
* MFA-Reset als eigener privilegierter Prozess.

## 6.8 Brute Force

Adaptive/exponentielle Limits nach:

```text
Account
IP
Subnet
Auth method
```

## 6.9 Service Accounts

Maschinen erhalten eigene Service Accounts mit scoped Tokens, Rotation und eigenem Audit.

\---

# 7\. Secret \& Key Management

## 7.1 Root KEK

Bevorzugt:

```text
TPM/HSM-backed KEK
```

Fallback:

```text
lokaler zufälliger CSPRNG-KEK
```

## 7.2 Envelope Encryption

```text
Root KEK
-> wrapped DEK pro Secret
-> AES-256-GCM Ciphertext
```

Crypto-Metadaten:

```text
crypto\_version
algorithm
key\_id
wrapped\_dek
nonce
ciphertext
tag
created\_at
```

Crypto Agility bleibt vorgesehen; initial nur freigegebene Suite aktiv.

## 7.3 Secret Classes

Mindestens:

```text
AUTH\_SECRET
TOTP\_SECRET
SIP\_CREDENTIAL
TRUNK\_CREDENTIAL
DNS\_API\_TOKEN
ACME\_SECRET
SERVICE\_TOKEN
BACKUP\_KEY
PRIVATE\_KEY
```

Jede Klasse definiert Reveal, Rotation, Export, Retention, Audit und Scope.

## 7.4 Secret Broker

Secrets werden nicht global über Environment Variables verteilt.

Normal:

```text
Consumer -> local Secret Broker -> Policy -> Secret
```

Falls Datei erforderlich:

```text
tmpfs
0600
short lived
unlink after use
```

Sensible Rust-Werte benutzen Secret-/Zeroizing-Wrapper.

## 7.5 Rotation und Löschung

* policybasierte Rotation;
* KEK-Rotation durch Rewrap der DEKs;
* Crypto Erasure durch Entfernen des wrapped DEK;
* Secret Lifecycle vollständig auditiert.

\---

# 8\. Domain Model

Domain-Grenzen:

```text
Identity
Organization
Telephony
Routing
Scheduling
Media
Audit
```

Betrieb: Single Tenant, Schema tenant-aware.

## 8.1 Kernobjekte

Getrennte Entitäten:

```text
User
Extension
Device
SIP Credential
Number
ProviderProfile
TrunkInstance
Site
Department
RingGroup
Queue
IVR
Voicemail
MediaAsset
Contact
AddressBook
Schedule
RouteGraph
```

`User != Extension != Device`.

Funktionsnebenstellen ohne Benutzer sind zulässig.

## 8.2 Rufnummern

Extern:

```text
canonical
display
source representation
type
country
```

Canonical bevorzugt E.164, interne Extensions eigene Klasse.

## 8.3 Provider

```text
ProviderProfile
├── registrar rules
├── transport
├── codecs
├── number format
└── capabilities
     ↓
TrunkInstance
├── credentials
├── DIDs
├── overrides
└── routing
```

## 8.4 Routing

Routing ist ein **typisierter Graph**, kein Raw-Dialplan.

Validierung:

* Referenzen;
* Dead Ends;
* Zyklen;
* Security Policies;
* Capability;
* Simulation.

## 8.5 Scheduling

```text
Weekly Hours
Holidays
Exceptions
Special Openings
Timezone
Priority
```

## 8.6 Media

Content-addressed Store:

```text
/data/media/sha256/<prefix>/<sha256>
```

Media-Metadaten mindestens: ID, SHA-256, MIME, Duration, Scope, Purpose.

## 8.7 Calls

Normalisiertes append-only Call Event Ledger, daraus abgeleitete CDR/Statistiken.

FreeSWITCH-Rohereignisse sind nicht die fachliche Wahrheit.

## 8.8 Config Versioning

```text
Draft
-> Validate
-> immutable Published Snapshot
-> Compile
-> Runtime
```

Published Snapshots sind rollbackfähig und gehasht.

## 8.9 IDs und Events

* UUIDv7;
* Transactional Outbox;
* idempotente Consumer;
* Domain Event Schema Versioning.

\---

# 9\. Config Compiler \& FreeSWITCH Adapter

Pipeline:

```text
Desired State
-> Normalize
-> Schema/Domain/Reference Validation
-> Security Policy Validation
-> Routing Validation
-> Capability Validation
-> Typed Rust IR
-> Sofia/Directory/Dialplan/Media/ACL Compiler
```

XML wird strukturell erzeugt, nie per unsicherer String-Konkatenation.

## 9.1 Static / Dynamic

Statisch:

```text
Modules
Sofia Profiles
TLS
Core Settings
ESL
XML-Curl Bootstrap
```

Dynamisch:

```text
Directory
Users
Extensions
Dialplan
Routing
```

## 9.2 Deployment

```text
/data/config/generation-N/
current -> generation-N
```

Ablauf:

```text
compile
-> validate
-> atomic activate
-> impact-specific reload/rescan
-> health/smoke test
-> commit
```

Fehler:

```text
rollback previous generation
```

## 9.3 Runtime Adapter

* eigener FreeSWITCH-Adapter;
* dauerhafte ESL-Verbindung;
* normalisierte Domain Events;
* Published-Snapshot-Cache für XML lookups;
* kontinuierlicher Desired-vs-Actual Reconciler;
* Drift Detection;
* Generation Manifest mit Hashes.

Nur explizit erlaubte FreeSWITCH-Module werden gebaut/geladen.

\---

# 10\. SIP Provider \& Trunks

## 10.1 Provider Catalog

* eingebaute geprüfte Templates;
* unabhängige signierte/versionierte Katalogupdates;
* deklarative Daten, kein Fremdcode;
* bestehende Trunks nie ungefragt migrieren;
* Generic SIP immer verfügbar.

## 10.2 Auth Capabilities

Unterstützt:

```text
Digest + REGISTER
Digest without REGISTER
IP Authentication
mTLS / Client Certificate
Provider-specific combinations
```

## 10.3 Inbound Trust

Provider-spezifische Kombination aus:

```text
Source ACL
Gateway Identity
Authentication
Transport Policy
DID Validation
Dedicated Context
Rate Limits
```

Verbindlich:

```text
UNKNOWN SOURCE -> DROP
UNKNOWN DID    -> DROP
```

TRUNK gelangt nie direkt in LOCAL.

## 10.4 Outbound Routing

Health-aware Primary/Secondary-Trunks.

Failover ist Cause-basiert, nicht pauschal bei jedem SIP-Fehler.

Normalized Causes mindestens:

```text
BUSY
NO\_ANSWER
REJECTED
UNREACHABLE
AUTH\_FAILED
PROVIDER\_FAILURE
NETWORK\_FAILURE
INVALID\_NUMBER
POLICY\_BLOCKED
```

## 10.5 Provider Defaults

Unterstützt:

* DNS/NAPTR/SRV/A/AAAA;
* TTL-respektierende Resolver;
* SIP OPTIONS Health;
* begrenztes Backoff;
* Header-Normalisierung;
* PAI/Privacy/From Policies;
* Provider-spezifische Nummernformatierung;
* Codec Allowlist;
* RFC 4733 DTMF bevorzugt.

\---

# 11\. Endgeräte \& Provisioning

## 11.1 Enrollment

Alle Modi unterstützt:

```text
A MAC/Serial only
B MAC/Serial + One-Time Enrollment Token \[DEFAULT]
C mTLS / Device Certificate
```

## 11.2 Transport

Default:

```text
HTTPS provisioning
```

HTTP/TFTP muss als expliziter Kompatibilitätsmodus möglich bleiben.

Legacy-Modus:

* sichtbar;
* lokale/definierte Netze bevorzugt;
* nie als HTTPS-gleichwertig darstellen;
* Config Encryption nutzen, falls Gerät es unterstützt.

## 11.3 Discovery

Gestuft:

```text
Local PnP
-> DHCP provisioning
-> Vendor RPS/Redirect optional
-> QR/Enrollment URL
-> manual URL
```

Keine Vendor-Cloud ist Pflicht.

## 11.4 Device Catalog

Signierter/versionierter deklarativer Geräte-/Firmwarekatalog.

Capabilities mindestens:

```text
Lines
BLF
DSS Keys
HTTPS
HTTP
TFTP
mTLS
SRTP
SIP TLS
Codecs
Directory
Firmware Update
Config Encryption
PnP
RPS
```

## 11.5 Firmware

Managed Policy:

```text
Inform
Recommended
Pin
Scheduled Update
Staged Rollout
```

Kein automatisches „always latest“.

## 11.6 Geräte-Adminpasswort

Ein gemeinsames starkes zufälliges Adminpasswort für alle verwalteten Telefone.

Pflicht:

* kein Herstellerdefault;
* im KMS;
* Anzeige nur nach Step-up;
* transaktionale Rotation über alle Geräte;
* pro Gerät Rolloutstatus;
* HTTPS-Admin bevorzugt;
* Telnet/SSH/SNMP deaktivieren oder absichern, wenn unnötig.

Trade-off: Kompromittierung eines Geräts kann den Blast Radius auf weitere Geräte erweitern.

\---

# 12\. Groundwire / Acrobits / Mobile

## 12.1 Betriebsarten

```text
Push via Acrobits SIPIS
Direct public SIP
LAN
VPN
```

Push ist optional.

## 12.2 Credentials

Eigenes SIP-Credential pro Mobile Device.

## 12.3 PUBLIC/PUSH Exposure

PUBLIC/PUSH darf aus dem gesamten Internet erreichbar sein.

Daher zwingend:

* TLS/SRTP;
* starke Device-Credentials;
* keine Guest Calls;
* Auth-/Request-Rate-Limits;
* per-IP/per-Account Abuse Protection;
* strikte Kontexttrennung;
* Audit;
* Security Dashboard.

Keine verpflichtende Acrobits-IP-Allowlist.

## 12.4 Push Privacy

Push wird pro Gerät explizit aktiviert; UI weist auf Acrobits SIPIS als externe Trust Boundary hin.

## 12.5 Ohne Public SIP

Groundwire bleibt per LAN/VPN/Direct nutzbar. Keine eigene EXTELIO Relay Cloud.

\---

# 13\. Home-Assistant-Integration

Verbindlich:

```text
bidirectional
single app
homeassistant\_api: true
hassio\_api: false
docker\_api: false
full\_access: false
```

Kein Ingress.

HA besitzt eigenen Service Principal.

HA darf operative Allowlist-Aktionen:

* DND;
* Rufumleitung;
* Tag/Nacht;
* Queue Login/Logout;
* Präsenz;
* Routing-Profil;
* definierte Ansage.

HA darf nicht:

* Secrets lesen;
* Adminrechte ändern;
* KMS ändern;
* Security Mode senken;
* Benutzer löschen;
* kritische Netzwerk-/TLS-Sicherheit verändern.

HA-Ausfall beeinträchtigt Telefonie nicht.

## 13.1 Privacy Boundary

**Keine personenbezogenen Telefoniedaten an Home Assistant.**

Erlaubt:

* PBX online;
* Trunk Health;
* Anzahl aktiver Gespräche;
* aggregierte Systemzustände;
* nicht-personenbezogene Queue-/Integrationszustände.

Nicht erlaubt:

* Rufnummern;
* Namen;
* personenbezogene Extensions;
* personenbezogene Call IDs/CDR;
* Voicemail-/Recording-Inhalte.

\---

# 14\. Privacy, Recording, Voicemail \& Audit

## 14.1 Retention

Zweckgebundene Policies getrennt für:

```text
Call Events
CDR
Statistics
Security Audit
System Logs
Voicemail
Recordings
```

Keine unbegrenzte Default-Retention.

## 14.2 Recording

Gesprächsaufzeichnung ist eine reguläre PBX-Funktion.

Unterstützt:

```text
manual
automatic per Queue
automatic per Number
rule-based
time/group-based
```

Pflicht:

* verschlüsselt;
* RBAC;
* Playback/Export/Delete auditiert;
* eigene Retention;
* Compliance-Hinweis;
* kein Recordinginhalt in Logs.

## 14.3 Voicemail

Verschlüsselte Speicherung.

Zugriff:

```text
Mailbox owner
Delegates
Systemadministrator mit expliziter Permission
```

Adminzugriff ist privilegiert und auditiert.

## 14.4 Audit

Security Audit getrennt von CDR/Systemlogs.

Manipulationsschutz:

```text
hash-chained append-only ledger
+ periodische HMAC/Signatur-Checkpoints
```

Audit von Secrets mindestens:

```text
CREATE
USE
REVEAL
ROTATE
REVOKE
DELETE
EXPORT
FAILED\_ACCESS
```

Nie Secretwerte protokollieren.

## 14.5 Logging

Nie loggen:

* Passwörter;
* Tokens;
* TOTP-Seeds;
* Private Keys;
* vollständige Authorization Header;
* Audioinhalte.

Diagnose-Rufnummern/IPs soweit möglich maskieren/pseudonymisieren.

Privacy Control Center ist Bestandteil der UI.

\---

# 15\. Backup, Restore \& Disaster Recovery

Zwei Backupwege:

```text
Home Assistant Backup
+
portable encrypted EXTELIO Backup
```

## 15.1 Hot Backup

Application-consistent:

```text
publish freeze
-> SQLite snapshot/checkpoint
-> Outbox/Audit sync
-> Manifest
-> backup
-> unfreeze
```

Laufende Calls sollen nicht beendet werden.

## 15.2 Restore Units

Mindestens:

```text
SYSTEM CONFIGURATION
DEVICE CONFIGURATION
MEDIA
CALL HISTORY
AUDIT
CONTACTS
```

Dependency Resolver verhindert inkonsistente Teilrestores.

## 15.3 Hardwarewechsel

Portable Backup-Verschlüsselung erlaubt Restore auf neuer Hardware und Re-Sealing unter neuem TPM/HSM-/Software-KEK.

## 15.4 Versionen

Backup enthält:

```text
backup\_format\_version
pbx\_version
schema\_version
crypto\_version
domain\_schema\_version
compiler\_version
device\_catalog\_version
provider\_catalog\_version
```

Migration schrittweise und explizit.

## 15.5 Restore Gate

```text
Integrity
-> Crypto
-> Migration
-> Network
-> Provider/Certificate
-> Compile
-> Smoke Test
```

Bei relevanter Abweichung: `RECOVERY MODE`.

\---

# 16\. Release, Supply Chain \& Security

## 16.1 Releases

Öffentlicher Kanal:

```text
Stable only
```

Keine Beta-/Nightly-Endnutzerkanäle.

Update:

```text
detect
-> signature/provenance verify
-> release notes
-> admin install
```

Optionale automatische Stable-Security-Updates im Wartungsfenster.

## 16.2 Trust Chain

Jedes Stable Release:

* OCI Digests;
* CycloneDX 1.7 SBOM;
* GitHub Artifact Attestation;
* Cosign Signature;
* SLSA-orientierte Provenance;
* Test Summary;
* Security Scan Summary;
* Migration Notes;
* Compliance Summary.

## 16.3 Dependency Security

Pflicht:

```text
Cargo.lock
package-lock.json
Base Image Digest Pinning
GitHub Actions SHA Pinning
SAST
Dependency Scan
License Scan
Secret Scan
Container Scan
```

Runtime-Image enthält keine Compiler/Buildtoolchains.

## 16.4 Security Verification

Ziel:

```text
OWASP ASVS 5.0.0
Level 2 vollständig
+ ausgewählte Level-3-Anforderungen
```

Defense in Depth:

* AppArmor;
* minimale Capabilities;
* getrennte Unix IDs;
* restriktive Dateirechte;
* tmpfs für Secrets;
* kein Docker Socket;
* kein NET\_ADMIN/SYS\_ADMIN.

## 16.5 Abuse Protection

Lokale adaptive Engine für:

```text
SIP REGISTER
SIP INVITE
Web Login
Password Reset
API Tokens
Provisioning
```

Reaktionen:

```text
ALLOW
DELAY
THROTTLE
TEMP\_BLOCK
```

## 16.6 Testing

Pflicht:

* Unit;
* Integration;
* Contract;
* Negative Tests;
* Trust-Boundary Fuzzing;
* SIP Black-Box/adversarial Tests;
* Backup/Restore;
* Security Regression Tests.

Vor 1.0 Stable: unabhängiger Penetrationstest.

PSIRT:

```text
SECURITY.md
Private Disclosure
Supported Versions
Security Advisory
CVE where appropriate
Signed Security Release
Root Cause Analysis
Regression Test
```

\---

# 17\. Compliance Evidence

Zentrale Matrix:

```text
COMP-REQ-ID
Standard
Requirement Reference
Applicability
Responsibility
PBX Requirement
Architecture Decision
Implementation Component
Verification Method
Automated Test ID
Manual Test ID
Evidence
Release Status
Exception / Risk Acceptance
```

Regel:

> Kein Control gilt als erfüllt, wenn Implementierung und belastbare Verifikation fehlen.

Öffentlich pro Stable Release: Compliance Summary.  
Vollständige Evidence Matrix: projektintern.

\---

# 18\. Monitoring, Health \& Self-Healing

Health States:

```text
HEALTHY
DEGRADED
UNHEALTHY
RECOVERY
UNSAFE\_OVERRIDE\_ACTIVE
MAINTENANCE
```

Pflichtchecks mindestens:

* pbx-web;
* pbx-core;
* pbx-worker;
* Secret Broker;
* SQLite;
* KMS;
* FreeSWITCH;
* Sofia LOCAL/PUBLIC/TRUNK;
* XML Adapter;
* TLS/Certificates;
* Media Store;
* Storage;
* Audit Ledger;
* Backup Status;
* HA Integration.

Defaultwarnungen:

```text
free storage <20% = Warning
free storage <10% or <1 GiB = Critical
certificate <=30d = Warning
<=14d = High
<=7d = Critical
```

## 18.1 Self-Healing

Kontrolliert und begrenzt:

* Worker restart;
* ESL reconnect;
* XML adapter recovery;
* Trunk re-registration;
* letzte valide Config Generation;
* Outbox retry.

Kein automatisches Ändern des fachlichen Desired State.

## 18.2 Hard-Error Override

Systemadministrator darf Hard Errors bewusst übersteuern.

Pflicht:

```text
technical detail
-> step-up auth
-> explicit warning
-> required reason
-> audit
-> UNSAFE\_OVERRIDE\_ACTIVE
```

Nicht übersteuerbar sind technisch nicht ausführbare Zustände, z. B. unlesbare DB, nicht entschlüsselbare erforderliche Secrets oder unmögliche Socket-Bindings.

\---

# 19\. Home-Assistant-App-Konfiguration

Zielzustand:

```yaml
name: "EXTELIO"
slug: "extelio"
arch:
  - amd64
  - aarch64
startup: services
boot: auto
init: false
host\_network: true
homeassistant\_api: true
hassio\_api: false
docker\_api: false
full\_access: false
ingress: false
tmpfs: true
backup: hot
stage: stable
```

Custom AppArmor ist Pflicht.

`stdin: true` nur, wenn der HA→EXTELIO Command Adapter darüber real implementiert wird.

## 19.1 Bootstrap Options

Mindestens:

```text
web\_mode
web\_http\_port
web\_https\_port

sip\_bind\_mode
sip\_bind\_ip

local\_sip\_port
local\_sips\_port
public\_sip\_port
public\_sips\_port
trunk\_sip\_port
trunk\_sips\_port

rtp\_start\_port
rtp\_end\_port

ipv6\_enabled
public\_push\_enabled
```

Keine langfristigen Secrets in `options.json`.

\---

# 20\. First Boot

Ablauf:

```text
Install EXTELIO
-> configure bootstrap networking
-> start
-> local preflight
-> create first admin
-> set local password
-> configure TOTP
-> initialize KMS
-> configure TLS/ACME
-> enroll Passkey when Secure Context available
-> configure provider
-> configure numbers
-> configure people/extensions/devices
-> routing
-> test
-> publish
```

Keine Default-Credentials.

Certbot/DNS-API-Secrets werden erst nach KMS-Initialisierung gespeichert.

\---

# 21\. Service- und Recovery-Runbook

Startabhängigkeiten:

```text
pbx-bootstrap
-> secret-broker
-> pbx-core
-> pbx-web / pbx-worker / pbx-xml-adapter
-> FreeSWITCH
-> reconciler / health
```

Recovery Mode aktiv:

* UI;
* DB-/Migration-Diagnose;
* KMS;
* Config Compiler;
* Audit/Health;
* Backup/Restore.

Recovery Mode standardmäßig aus:

* Trunk Registration;
* Outbound Calls;
* PUBLIC/PUSH;
* Auto Provisioning;
* produktive Auto-Aktivierung.

Pflicht-Runbooks:

1. Provider/Trunk offline
2. ACME/Zertifikatfehler
3. SIP Brute Force
4. DB Integrity Error
5. TPM/KMS unavailable
6. Config Generation corrupt
7. FreeSWITCH start failure
8. XML/ESL failure
9. Storage critical
10. HA Core unavailable
11. Groundwire Push unavailable
12. Restore new hardware
13. Admin recovery
14. SIP credential compromise
15. shared device-admin credential compromise
16. catalog corruption
17. failed update/rollback
18. `UNSAFE\_OVERRIDE\_ACTIVE`

Je Runbook:

```text
Symptom
Detection
Impact
Immediate Action
Safe Recovery
Verification
Rollback
Audit/Evidence
```

\---

# 22\. Repository und Build

## 22.1 Architektur-Freeze Baseline

|Komponente|Baseline|
|-|-|
|HA Base|Alpine 3.24 / docker-base 2026.08.0|
|Arch|amd64, aarch64|
|Rust|1.98.1 stable|
|Backend|Axum, exakte Version via Cargo.lock|
|Node|24.21.0 LTS|
|Frontend|React + TypeScript strict|
|FreeSWITCH|1.11.3|
|s6|aus HA Base|
|Certbot|5.7.0|
|DB|SQLite + WAL|
|AEAD|AES-256-GCM|
|SBOM|CycloneDX 1.7|
|Release|Stable only|

HA Base Digests:

```text
amd64
ghcr.io/home-assistant/amd64-base:3.24-2026.08.0
sha256:db83d263152a02d6daf3415a73132b2386f43a03a9c248bdaabe047b067c8eea

aarch64
ghcr.io/home-assistant/aarch64-base:3.24-2026.08.0
sha256:265232fe91fb4271f75f17ebd4bb0749ab24fc8a64d8db38ddaccd460e9580ee
```

Dockerfile ist Build-Source-of-Truth; kein neues `build.yaml`.

## 22.2 Kanonische Repository-Struktur

```text
extelio-repository/
├── repository.yaml
├── README.md
├── LICENSE
├── SECURITY.md
├── docs/
│   ├── EXTELIO\_IMPLEMENTIERUNGSSPEZIFIKATION.md
│   └── design/
│       └── EXTELIO\_BRANDBOARD.png
├── .github/
│   └── workflows/
│       ├── quality.yml
│       ├── security.yml
│       ├── build.yml
│       ├── release.yml
│       └── attest.yml
└── extelio/
    ├── config.yaml
    ├── Dockerfile
    ├── apparmor.txt
    ├── README.md
    ├── DOCS.md
    ├── CHANGELOG.md
    ├── icon.png
    ├── logo.png
    ├── build-lock.json
    ├── Cargo.toml
    ├── Cargo.lock
    ├── rust-toolchain.toml
    ├── package.json
    ├── package-lock.json
    ├── tsconfig.json
    ├── vite.config.ts
    ├── brand/
    ├── src/
    │   ├── rust/
    │   │   ├── domain/
    │   │   ├── application/
    │   │   ├── infrastructure/
    │   │   ├── adapters/
    │   │   ├── auth/
    │   │   ├── kms/
    │   │   ├── compiler/
    │   │   ├── backup/
    │   │   ├── audit/
    │   │   ├── health/
    │   │   └── bin/
    │   └── web/
    ├── migrations/
    ├── schemas/
    ├── catalogs/
    │   ├── providers/
    │   └── devices/
    ├── freeswitch/
    ├── rootfs/
    ├── tests/
    ├── compliance/
    └── scripts/
```

## 22.3 Multi-Stage Build

```text
rust-builder
web-builder
freeswitch-builder
certbot-builder
runtime
```

Runtime enthält nur erforderliche Runtime-Artefakte.

## 22.4 FreeSWITCH Allowlist

Mindestens evaluieren/pinnen:

```text
mod\_sofia
mod\_event\_socket
mod\_xml\_curl
mod\_commands
mod\_dptools
mod\_db
mod\_hash
mod\_loopback
mod\_tone\_stream
mod\_sndfile
mod\_voicemail
mod\_callcenter
mod\_opus
```

Nicht standardmäßig:

```text
mod\_xml\_rpc
mod\_verto
mod\_rtmp
unnötige Script Engines
Debug/Demo Module
unnötige Netzwerkdienste
```

## 22.5 Persistentes Layout

```text
/data/
├── db/pbx.sqlite3
├── config/generation-\*/
├── config/current
├── media/sha256/
├── audit/
├── backup/
├── certs/
├── kms/
├── catalogs/
├── state/
└── diagnostics/
```

Temporäre Secrets nur tmpfs-basiert unter `/run`/`/tmp`.

\---

# 23\. Rebuild und Release Gate

Lokaler/CI-Kernpfad:

```bash
git clone <CANONICAL\_REPOSITORY\_URL>
cd extelio-repository/extelio

./scripts/verify-locks.sh
./scripts/test.sh
./scripts/build.sh --platform linux/amd64
./scripts/build.sh --platform linux/arm64
./scripts/sbom.sh
./scripts/sign.sh
```

Rust:

```bash
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked
```

Web:

```bash
npm ci
npm run typecheck
npm run lint
npm test
npm run build
```

Stable Release benötigt:

```text
Git Tag + Commit
OCI Multi-Arch Digest
per-Arch Digests
build-lock.json
Cargo.lock
package-lock.json
CycloneDX SBOM
Provenance
GitHub Attestation
Cosign Signature
Test Summary
Security Scan Summary
Migration Notes
Compliance Summary
Release Notes
```

Reproducibility States:

```text
ARCHITECTURE\_COMPLETE
IMPLEMENTATION\_LOCK\_PENDING
REPRODUCIBLE
REPRODUCIBILITY\_FAILED
```

Aktuell:

```text
ARCHITECTURE\_COMPLETE
IMPLEMENTATION\_LOCK\_PENDING
```

`REPRODUCIBLE` erst wenn reales Repository, Lockfiles, Digests, Clean Builds für amd64/aarch64, Restore-Test, SBOM/Provenance/Signatur und Compliance Evidence erfolgreich vorliegen.

\---

# 24\. Definition of Done für die erste lauffähige Version

Die erste implementierbare EXTELIO-Version gilt technisch als erreicht, wenn mindestens:

1. HA-App auf amd64 und aarch64 baut und startet.
2. Eigenes EXTELIO-Web-UI erscheint im Designsystem.
3. First-Admin-Enrollment funktioniert.
4. Passwort + TOTP funktionieren; Passkey bei Secure Context.
5. SQLite/KMS/Secret Store funktionieren.
6. FreeSWITCH startet mit drei Sofia-Profilen.
7. LOCAL-Gerät kann registrieren.
8. SIP-Trunk kann angelegt und registriert/authentifiziert werden.
9. Ein- und ausgehender Testcall funktioniert.
10. Typisierter Routing Flow kann gespeichert, simuliert, kompiliert und aktiviert werden.
11. Generation-Rollback funktioniert.
12. Provisioning eines unterstützten Generic-/Referenzgeräts funktioniert.
13. Groundwire Direct funktioniert; Pushpfad ist architektonisch vorbereitet/testbar.
14. HA-Integration liefert nur nicht-personenbezogene Health-/Betriebszustände.
15. Recording und Voicemail sind verschlüsselt und permission-basiert.
16. Hot Backup und Restore funktionieren.
17. Security/Health Dashboard zeigt reale Zustände.
18. Support Bundle redigiert Secrets/PII.
19. CI erzeugt Tests, SBOM, Scans, Provenance/Attestation/Signatur.
20. Compliance-Matrix enthält für implementierte Controls Evidence.

\---

# 25\. Kanonische Design-/Projektartefakte

Diese Spezifikation enthält den finalen Zustand. Zusätzlich gehören zum Projekt:

```text
EXTELIO\_IMPLEMENTIERUNGSSPEZIFIKATION.md
EXTELIO\_BRANDBOARD.png
```

Die historische Entscheidungsfassung bleibt archiviert, ist aber **nicht** mehr Implementierungs-Source-of-Truth.

\---

# 26\. Änderungsregel

Neue technische Einzelentscheidungen sind keine Nutzerabstimmung, solange sie den hier definierten Zustand nicht ändern.

Eine neue Architekturentscheidung ist nur erforderlich, wenn eine Änderung mindestens einen dieser Punkte verändert:

* Trust Boundary;
* Datenmodell;
* Security-/Privacy-Garantie;
* externe Schnittstelle;
* Installations-/Recovery-Modell;
* wesentliche Nutzerfunktion;
* Interoperabilität;
* schwer reversible Technologieentscheidung.

Ansonsten gilt:

> Implementieren, testen, dokumentieren – keine künstliche Entscheidungsrunde.

