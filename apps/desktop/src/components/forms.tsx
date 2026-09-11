import { useEffect, useState, type FormEvent, type ReactNode } from "react";

import { ApiRequestError } from "@/api/client";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

export function errorMessage(e: unknown, fallback = "Something went wrong."): string {
  return e instanceof ApiRequestError ? e.message : e instanceof Error ? e.message : fallback;
}

/** Label + control + hint, laid out consistently. */
export function Field({
  label,
  htmlFor,
  hint,
  children,
  className,
}: {
  label: string;
  htmlFor?: string;
  hint?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      {hint && <span className="text-[12px] text-muted-foreground">{hint}</span>}
    </div>
  );
}

export const selectClass =
  "h-8 w-full rounded-lg border border-input bg-background px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50";

export function TextField(props: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  required?: boolean;
  placeholder?: string;
  hint?: string;
  autoFocus?: boolean;
  className?: string;
}) {
  return (
    <Field label={props.label} htmlFor={props.id} hint={props.hint} className={props.className}>
      <Input
        id={props.id}
        type={props.type ?? "text"}
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
        required={props.required}
        placeholder={props.placeholder}
        autoFocus={props.autoFocus}
      />
    </Field>
  );
}

export function TextAreaField(props: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  rows?: number;
  className?: string;
}) {
  return (
    <Field label={props.label} htmlFor={props.id} className={props.className}>
      <Textarea id={props.id} value={props.value} onChange={(e) => props.onChange(e.target.value)} rows={props.rows ?? 3} />
    </Field>
  );
}

export function SelectField<T extends string>(props: {
  id: string;
  label: string;
  value: T | "";
  onChange: (v: T | "") => void;
  options: { value: T; label: string }[];
  placeholder?: string;
  required?: boolean;
  disabled?: boolean;
  hint?: string;
  className?: string;
}) {
  return (
    <Field label={props.label} htmlFor={props.id} hint={props.hint} className={props.className}>
      <select
        id={props.id}
        className={selectClass}
        value={props.value}
        onChange={(e) => props.onChange(e.target.value as T | "")}
        required={props.required}
        disabled={props.disabled}
      >
        {props.placeholder !== undefined && <option value="">{props.placeholder}</option>}
        {props.options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </Field>
  );
}

/** Empty string ⇄ null conversion for optional text fields. */
export const opt = (s: string): string | null => (s.trim() === "" ? null : s.trim());
export const str = (s: string | null | undefined): string => s ?? "";

/**
 * Dialog with a form inside: handles busy state, error display and submit.
 * `onSubmit` throws to show an error; resolving closes the dialog.
 */
export function FormDialog({
  open,
  onOpenChange,
  title,
  description,
  submitLabel = "Save",
  onSubmit,
  children,
  wide,
  destructive,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  submitLabel?: string;
  onSubmit: () => Promise<void>;
  children: ReactNode;
  wide?: boolean;
  destructive?: boolean;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) setError(null);
  }, [open]);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await onSubmit();
      onOpenChange(false);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => onOpenChange(o)}>
      <DialogContent className={cn(wide && "sm:max-w-2xl")}>
        <form onSubmit={submit} className="flex flex-col gap-4">
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            {description && <DialogDescription>{description}</DialogDescription>}
          </DialogHeader>
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          {children}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={busy}>
              Cancel
            </Button>
            <Button type="submit" variant={destructive ? "destructive" : "default"} disabled={busy}>
              {busy ? "Saving…" : submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
