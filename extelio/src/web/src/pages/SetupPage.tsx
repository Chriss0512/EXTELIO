/**
 * First-Boot-Wizard - Kapitel 20.
 * Ablauf: Systemadministrator anlegen -> TOTP verbindlich einrichten ->
 * Anmeldung. Danach gibt es keinen offenen Zugang mehr.
 */
import { useState } from "react";
import { api, ApiError } from "../lib/api";
import { Button, Field, Notice, TextInput } from "../components/ui";
import { Wordmark } from "../components/Shell";

interface EnrollResponse {
  user_id: string;
  totp_secret: string;
  provisioning_uri: string;
}

export function SetupPage({ onDone }: { onDone: () => void }) {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [repeat, setRepeat] = useState("");
  const [enroll, setEnroll] = useState<EnrollResponse | null>(null);
  const [code, setCode] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function createAdmin() {
    setError("");
    if (password !== repeat) {
      setError("Die beiden Passwörter stimmen nicht überein.");
      return;
    }
    setBusy(true);
    try {
      const res = await api.post<EnrollResponse>("/setup/admin", {
        username,
        display_name: displayName,
        password,
      });
      setEnroll(res);
      setStep(2);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    } finally {
      setBusy(false);
    }
  }

  async function confirmTotp() {
    if (!enroll) return;
    setError("");
    setBusy(true);
    try {
      await api.post("/setup/totp/confirm", { user_id: enroll.user_id, code });
      setStep(3);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="centered-page">
      <div className="centered-card-wide panel">
        <div className="panel-header">
          <Wordmark />
          <span className="metadata">Erstinbetriebnahme · Schritt {step} von 3</span>
        </div>

        {step === 1 ? (
          <div className="panel-body stack">
            <div className="stack-sm">
              <h1 className="section-title">Systemadministrator anlegen</h1>
              <p className="metadata">
                Dieses Konto verwaltet die Anlage. Ein zweiter Faktor ist verpflichtend und wird
                im nächsten Schritt eingerichtet.
              </p>
            </div>
            {error ? <Notice tone="error">{error}</Notice> : null}
            <Field label="Anmeldename" help="3 bis 64 Zeichen, Buchstaben, Ziffern, Punkt, Bindestrich, Unterstrich.">
              <TextInput value={username} onChange={setUsername} autoComplete="username" placeholder="admin" />
            </Field>
            <Field label="Anzeigename">
              <TextInput value={displayName} onChange={setDisplayName} placeholder="Vorname Nachname" />
            </Field>
            <Field label="Passwort" help="Mindestens 12 Zeichen aus drei der vier Zeichenarten.">
              <TextInput value={password} onChange={setPassword} type="password" autoComplete="new-password" />
            </Field>
            <Field label="Passwort wiederholen">
              <TextInput value={repeat} onChange={setRepeat} type="password" autoComplete="new-password" />
            </Field>
            <div className="row">
              <div className="spacer" />
              <Button
                variant="primary"
                onClick={createAdmin}
                disabled={busy || !username || !displayName || !password}
              >
                Weiter
              </Button>
            </div>
          </div>
        ) : null}

        {step === 2 && enroll ? (
          <div className="panel-body stack">
            <div className="stack-sm">
              <h1 className="section-title">Zwei-Faktor-Authentifizierung einrichten</h1>
              <p className="metadata">
                Trage diesen Schlüssel in deiner Authenticator-App ein und bestätige mit dem
                erzeugten Code. Der Schlüssel wird danach nicht erneut angezeigt.
              </p>
            </div>
            {error ? <Notice tone="error">{error}</Notice> : null}
            <Field label="Schlüssel">
              <pre className="code-block">{enroll.totp_secret}</pre>
            </Field>
            <Field label="Einrichtungs-URI" help="Alternativ in der App als URI hinterlegen.">
              <pre className="code-block">{enroll.provisioning_uri}</pre>
            </Field>
            <Field label="Sechsstelliger Code">
              <TextInput value={code} onChange={setCode} inputMode="numeric" placeholder="123456" />
            </Field>
            <div className="row">
              <div className="spacer" />
              <Button variant="primary" onClick={confirmTotp} disabled={busy || code.length !== 6}>
                Bestätigen
              </Button>
            </div>
          </div>
        ) : null}

        {step === 3 ? (
          <div className="panel-body stack">
            <Notice tone="success">Die Erstinbetriebnahme ist abgeschlossen.</Notice>
            <p className="body-text">
              Melde dich jetzt mit deinem Anmeldenamen, Passwort und dem Code aus der
              Authenticator-App an.
            </p>
            <div className="row">
              <div className="spacer" />
              <Button variant="primary" onClick={onDone}>
                Zur Anmeldung
              </Button>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
