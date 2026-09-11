import { useState, type FormEvent } from "react";

import { ApiRequestError } from "@/api/client";
import { AuthFrame } from "@/components/AuthFrame";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useApp } from "@/lib/app-state";

export function BootstrapPage() {
  const { bootstrap, serverUrl } = useApp();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (password !== confirm) {
      setError("The two passwords do not match.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await bootstrap(name, email, password);
    } catch (err) {
      setError(err instanceof ApiRequestError ? err.message : "Could not create the administrator.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <AuthFrame
      title="Create the first administrator"
      description="This server has no users yet. The account you create here gets full access and can add the leasing, operations and management users in Settings."
      footer={`Connected to ${serverUrl}`}
    >
      <form onSubmit={submit} className="flex flex-col gap-4">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="bs-name">Full name</Label>
          <Input id="bs-name" value={name} onChange={(e) => setName(e.target.value)} autoFocus required />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="bs-email">Email</Label>
          <Input id="bs-email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="bs-password">Password</Label>
          <Input
            id="bs-password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            minLength={8}
            required
          />
          <span className="text-[12px] text-muted-foreground">At least 8 characters.</span>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="bs-confirm">Confirm password</Label>
          <Input id="bs-confirm" type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} required />
        </div>
        <Button type="submit" disabled={busy}>
          {busy ? "Creating…" : "Create administrator"}
        </Button>
      </form>
    </AuthFrame>
  );
}
