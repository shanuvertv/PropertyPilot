import { useEffect, useState, type FormEvent } from "react";

import { ApiRequestError } from "@/api/client";
import { AuthFrame } from "@/components/AuthFrame";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useApp } from "@/lib/app-state";
import { platform } from "@/lib/secure";

export function SetupPage() {
  const { serverUrl, configureServer, healthError } = useApp();
  const [url, setUrl] = useState(serverUrl || "http://localhost:8787");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [mobile, setMobile] = useState(false);

  // On a phone "localhost" is the phone itself: start empty and explain what to enter.
  useEffect(() => {
    platform()
      .then((os) => {
        if (os === "android" || os === "ios") {
          setMobile(true);
          if (!serverUrl) setUrl("");
        }
      })
      .catch(() => {});
  }, [serverUrl]);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await configureServer(url);
    } catch (err) {
      setError(err instanceof ApiRequestError ? err.message : "Could not connect to that address.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <AuthFrame
      title="Connect to your server"
      description="Enter the address of the PropertyPilot server your IT team provided. It is saved securely on this device."
      footer={
        mobile
          ? "Use the server's public address, e.g. https://renewals.yourcompany.com — or its LAN address such as http://192.168.1.10:8787 while on the office Wi-Fi."
          : "Example: http://leasing-server:8787 or https://renewals.yourcompany.com"
      }
    >
      <form onSubmit={submit} className="flex flex-col gap-4">
        {(error || (serverUrl && healthError)) && (
          <Alert variant="destructive">
            <AlertTitle>Cannot reach the server</AlertTitle>
            <AlertDescription>{error ?? healthError}</AlertDescription>
          </Alert>
        )}
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="server-url">Server address</Label>
          <Input
            id="server-url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder={mobile ? "https://renewals.yourcompany.com" : "http://server:8787"}
            inputMode="url"
            autoFocus
            autoComplete="off"
            spellCheck={false}
          />
        </div>
        <Button type="submit" disabled={busy || !url.trim()}>
          {busy ? "Connecting…" : "Connect"}
        </Button>
      </form>
    </AuthFrame>
  );
}
