# EXTELIO Add-on Repository

Software-defined Communications als Home-Assistant-App.

## Repository in Home Assistant einbinden

1. In Home Assistant **Einstellungen → Add-ons → Add-on Store** öffnen.
2. Über das Dreipunktmenü **Repositories** wählen.
3. Die Adresse dieses Repositories eintragen und hinzufügen.
4. EXTELIO erscheint anschließend als installierbares Add-on.

## Was EXTELIO ist

EXTELIO ist eine Telefonanlage mit eigener Weboberfläche und eigener
Anmeldung. Sie läuft nicht über Home-Assistant-Ingress, weil SIP und RTP
direkt am Host anliegen müssen und weil die Anlage eine von Home Assistant
unabhängige Rechteverwaltung braucht.

Nach außen soll die Bedienung einfach sein. Nach innen gelten strenge Regeln:
typisiertes Routing statt handgeschriebener Wählpläne, verschlüsselte
Secret-Verwaltung, ein fälschungssicheres Sicherheitsprotokoll und
Konfigurationsstände, die sich jederzeit zurückrollen lassen.

## Aufbau

```text
extelio-repository/
├── repository.yaml          Kennzeichnung als Add-on-Repository
├── LICENSE
├── SECURITY.md
├── docs/                    Spezifikation und Designreferenz
├── .github/workflows/       Qualität, Sicherheit, Build, Release, Nachweis
└── extelio/                 Die eigentliche App
    ├── config.yaml          Add-on-Konfiguration
    ├── build.yaml           Basisimages je Architektur
    ├── Dockerfile           Mehrstufiger Build
    ├── apparmor.txt         Verbindliches AppArmor-Profil
    ├── build-lock.json      Festgeschriebene Baseline
    ├── brand/               Marken- und Designartefakte
    ├── compliance/          Zuordnung Anforderung → Umsetzung → Nachweis
    ├── freeswitch/          Modul-Allowlist
    ├── migrations/          Datenbankschema
    ├── rootfs/              Dienste und Startreihenfolge
    ├── scripts/             Test, Build, SBOM, Signatur
    ├── src/rust/            Backend
    ├── src/web/             Weboberfläche
    └── tests/               End-to-End-Test der API
```

## Stand dieser Ausgabe

Vollständig nutzbar sind Erstinbetriebnahme, Anmeldung mit zweitem Faktor,
Benutzer- und Rechteverwaltung, Nebenstellen, Geräte, Rufnummern, Trunks,
Gruppen, Flows mit Prüfung und Simulation, Schlüsselverwaltung,
Konfigurationsgenerationen mit Rollback, Sicherheitsprotokoll, Systemstatus und
Sicherungen.

Der Telefonie-Core wird über den Build-Parameter `WITH_FREESWITCH` eingebunden
und ist in der Voreinstellung nicht enthalten. Details dazu stehen in
`extelio/DOCS.md`.

## Entwicklung

```bash
cd extelio
./scripts/test.sh          # Formatierung, Lint, Tests, Frontend-Build
./scripts/build.sh amd64   # Lokaler Image-Build
```
