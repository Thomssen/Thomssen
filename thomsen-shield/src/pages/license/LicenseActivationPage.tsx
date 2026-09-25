import { useState, type FormEvent } from "react";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Field, TextInput } from "@/components/ui/Field";
import { useLicense } from "@/state/LicenseProvider";
import { errorMessage } from "@/services/ipc";
import { APP } from "@/config/app";
import "./LicenseActivationPage.css";

/** Shown in place of the main app whenever there is no active license - see
 * `App.tsx`. The DEV license provider behind `useLicense()` accepts any
 * key matching `SHIELD-XXXX-XXXX-XXXX`; a real backend slots in later
 * behind the exact same `activate()` call - see src-tauri/src/license. */
export function LicenseActivationPage() {
  const { activate } = useLicense();
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!key.trim()) {
      setError("Enter your license key to continue.");
      return;
    }
    setError(null);
    setBusy(true);
    void activate(key.trim())
      .catch((err) => setError(errorMessage(err)))
      .finally(() => setBusy(false));
  };

  return (
    <div className="license-gate">
      <div className="license-gate__panel">
        <span className="license-gate__logo" aria-hidden="true">
          S
        </span>
        <p className="license-gate__brand">{APP.name}</p>
        <h1 className="license-gate__title">Activate License</h1>
        <p className="license-gate__lead">Enter the license key from your purchase confirmation to unlock Thomsen Shield.</p>

        <Card className="license-gate__card">
          <form onSubmit={onSubmit} className="license-gate__form">
            <Field label="License Key" error={error} htmlFor="license-key">
              <TextInput
                id="license-key"
                mono
                value={key}
                onChange={(e) => setKey(e.target.value.toUpperCase())}
                placeholder="TSHIELD-XXXX-XXXX-XXXX"
                autoFocus
              />
            </Field>
            <Button type="submit" variant="primary" fullWidth loading={busy}>
              Activate
            </Button>
          </form>
        </Card>

        <p className="license-gate__hint">Thomsen Shield is a one-time purchase - €50.00, lifetime. Don't have a key yet? Visit thomssen.dev to buy one.</p>
      </div>
    </div>
  );
}
