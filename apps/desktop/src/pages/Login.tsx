import { useState, type FormEvent } from "react";

import { ApiRequestError } from "@/api/client";
import { AuthFrame } from "@/components/AuthFrame";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useApp } from "@/lib/app-state";

export function LoginPage() {
  const { signIn, serverUrl, changeServer } = useApp();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await signIn(email, password);
    } catch (err) {
      setError(err instanceof ApiRequestError ? err.message : "Sign-in failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <AuthFrame
      title="Sign in"
      footer={
        <>
          Connected to {serverUrl} ·{" "}
          <button type="button" className="underline underline-offset-2 hover:text-foreground" onClick={() => void changeServer()}>
            change server
          </button>
        </>
      }
    >
      <form onSubmit={submit} className="flex flex-col gap-4">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="login-email">Email</Label>
          <Input
            id="login-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            autoFocus
            autoComplete="username"
            required
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="login-password">Password</Label>
          <Input
            id="login-password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            required
          />
        </div>
        <Button type="submit" disabled={busy}>
          {busy ? "Signing in…" : "Sign in"}
        </Button>
      </form>
    </AuthFrame>
  );
}
