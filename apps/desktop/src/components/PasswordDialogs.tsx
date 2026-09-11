import { useState } from "react";

import { FormDialog, TextField } from "@/components/forms";
import { useApp } from "@/lib/app-state";

const MIN_LENGTH = 8;

function checkNew(password: string, confirm: string): string | null {
  if (password.length < MIN_LENGTH) return `Use at least ${MIN_LENGTH} characters.`;
  if (password !== confirm) return "The two new passwords do not match.";
  return null;
}

/** The signed-in user changes their own password; other devices are signed out. */
export function ChangePasswordDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const { api } = useApp();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");

  async function submit() {
    const problem = checkNew(next, confirm);
    if (problem) throw new Error(problem);
    await api.changePassword(current, next);
    setCurrent("");
    setNext("");
    setConfirm("");
  }

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Change password"
      description="Your other signed-in devices will be signed out."
      submitLabel="Change password"
      onSubmit={submit}
    >
      <TextField id="pw-current" label="Current password" type="password" value={current} onChange={setCurrent} required autoFocus />
      <TextField id="pw-new" label="New password" type="password" value={next} onChange={setNext} required hint={`At least ${MIN_LENGTH} characters.`} />
      <TextField id="pw-confirm" label="Repeat new password" type="password" value={confirm} onChange={setConfirm} required />
    </FormDialog>
  );
}

/** Admin sets a temporary password for another user; that user is signed out everywhere. */
export function ResetPasswordDialog({
  open,
  onOpenChange,
  user,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  user: { id: string; name: string } | null;
}) {
  const { api } = useApp();
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");

  async function submit() {
    if (!user) return;
    const problem = checkNew(next, confirm);
    if (problem) throw new Error(problem);
    await api.resetPassword(user.id, next);
    setNext("");
    setConfirm("");
  }

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={user ? `Reset password for ${user.name}` : "Reset password"}
      description="Share the temporary password out of band. They are signed out everywhere and can change it after signing in."
      submitLabel="Reset password"
      onSubmit={submit}
    >
      <TextField id="reset-new" label="Temporary password" type="password" value={next} onChange={setNext} required autoFocus hint={`At least ${MIN_LENGTH} characters.`} />
      <TextField id="reset-confirm" label="Repeat password" type="password" value={confirm} onChange={setConfirm} required />
    </FormDialog>
  );
}
