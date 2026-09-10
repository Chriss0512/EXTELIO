/**
 * Anmeldung - Kapitel 6.1.
 * Passwort und TOTP. Passkeys sind laut Spezifikation der bevorzugte Weg und
 * werden sichtbar angekuendigt, sobald ein Secure Context vorliegt.
 */
import { useState } from "react";
import { api, ApiError } from "../lib/api";
import { Button, Field, Notice, TextInput } from "../components/ui";
import { Wordmark } from "../components/Shell";

export function LoginPage({
  onSuccess,
  secureContext,
}: {
  onSuccess: () => void;
  secureContext: boolean;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [totp, setTotp] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit() {
    setError("");
    setBusy(true);
    try {
      await api.post("/auth/login", { username, password, totp });
      onSuccess();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="centered-page">
      <div className="centered-card panel">
        <div className="panel-header">
          <Wordmark />
        </div>
        <div className="panel-body stack">
          <h1 className="section-title">Anmelden</h1>
          {error ? <Notice tone="error">{error}</Notice> : null}
          {!secureContext ? (
            <Notice tone="warning">
              Diese Verbindung ist kein sicherer Kontext. Passkeys bleiben deaktiviert, bis ein
              Hostname mit HTTPS eingerichtet ist.
            </Notice>
          ) : null}
          <Field label="Anmeldename">
            <TextInput value={username} onChange={setUsername} autoComplete="username" />
          </Field>
          <Field label="Passwort">
            <TextInput value={password} onChange={setPassword} type="password" autoComplete="current-password" />
          </Field>
          <Field label="Code aus der Authenticator-App">
            <TextInput value={totp} onChange={setTotp} inputMode="numeric" placeholder="123456" />
          </Field>
          <Button variant="primary" onClick={submit} disabled={busy || !username || !password}>
            Anmelden
          </Button>
        </div>
      </div>
    </div>
  );
}
