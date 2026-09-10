# Sicherheit

## Schwachstellen melden

Sicherheitsrelevante Funde bitte nicht als oeffentliches Issue eroeffnen.
Meldungen gehen an die im Repository hinterlegte Kontaktadresse. Wir bestaetigen
den Eingang und melden uns mit einer ersten Einschaetzung zurueck.

## Geltungsbereich

EXTELIO ist eine Telefonanlage. Sicherheitsrelevant sind insbesondere:

- Authentifizierung, Sitzungen und Rechtevergabe (Kapitel 6)
- Secret- und Schluesselverwaltung (Kapitel 7)
- SIP-Profile, ACLs und Transportsicherheit (Kapitel 5)
- Erzeugung der FreeSWITCH-Konfiguration (Kapitel 9)
- Audit-Ledger und dessen Nachweiskette (Kapitel 14.4)

## Was EXTELIO zusagt

- Passwoerter werden mit Argon2id und individuellem Salt gespeichert.
- Ein hardwaregestuetzter Pepper wird nur verwendet, wenn TPM oder HSM
  verfuegbar sind. Einen softwarebasierten Ersatz-Pepper gibt es bewusst nicht.
- Secrets liegen ausschliesslich verschluesselt vor (AES-256-GCM, Envelope
  Encryption). Sie werden nicht ueber Umgebungsvariablen verteilt.
- Passwoerter, Tokens, TOTP-Seeds, private Schluessel und Audioinhalte werden
  nie protokolliert.
- Es gibt keine Standardzugangsdaten. Der erste Administrator wird bei der
  Erstinbetriebnahme angelegt und muss einen zweiten Faktor einrichten.

## Compliance-Anspruch

EXTELIO ist auf `certification-ready` ausgelegt, nicht auf eine
Zertifizierungsbehauptung. Die Zuordnung von Anforderungen zu Umsetzung und
Nachweis steht in `extelio/compliance/requirements.yaml`.
