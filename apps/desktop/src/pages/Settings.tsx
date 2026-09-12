import { useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiRequestError } from "@/api/client";
import { ROLE_LABEL, ROLES, type Role } from "@/api/types";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useApp } from "@/lib/app-state";
import { ChecklistTemplateCard } from "@/pages/settings/ChecklistTemplateCard";
import { EmailTemplatesCard } from "@/pages/settings/EmailTemplatesCard";
import { OrganisationCard, ReminderScheduleCard } from "@/pages/settings/AutomationCards";
import { AuditCard } from "@/pages/settings/AuditCard";
import { ImportCard } from "@/pages/settings/ImportCard";
import { MailSettingsCard } from "@/pages/settings/MailSettingsCard";
import { ResetPasswordDialog } from "@/components/PasswordDialogs";

function formatWhen(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

export function SettingsPage() {
  const { api, health, session } = useApp();
  const queryClient = useQueryClient();

  const users = useQuery({ queryKey: ["users"], queryFn: () => api.listUsers() });
  const system = useQuery({ queryKey: ["system-status"], queryFn: () => api.systemStatus(), refetchInterval: 30_000 });

  const [form, setForm] = useState({ name: "", email: "", role: "LEASING" as Role, password: "" });
  const [resetFor, setResetFor] = useState<{ id: string; name: string } | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const createUser = useMutation({
    mutationFn: () => api.createUser(form),
    onSuccess: () => {
      setForm({ name: "", email: "", role: "LEASING", password: "" });
      setFormError(null);
      void queryClient.invalidateQueries({ queryKey: ["users"] });
    },
    onError: (e) => setFormError(e instanceof ApiRequestError ? e.message : "Could not create the user."),
  });

  const toggleActive = useMutation({
    mutationFn: ({ id, active }: { id: string; active: boolean }) => api.setUserActive(id, active),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["users"] }),
  });

  function submit(e: FormEvent) {
    e.preventDefault();
    createUser.mutate();
  }

  const scheduler = system.data?.scheduler;

  return (
    <>
      <PageHeader title="Settings" description="Users and roles, tenant-list import, renewal checklist, email sending and templates, automation." />

      <div className="grid grid-cols-[minmax(0,1fr)] gap-5 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="flex min-w-0 flex-col gap-5">
          <Card>
            <CardHeader>
              <CardTitle>Users</CardTitle>
              <CardDescription>Deactivating a user signs them out everywhere immediately.</CardDescription>
            </CardHeader>
            <CardContent>
              {users.isError && (
                <Alert variant="destructive" className="mb-3">
                  <AlertDescription>
                    {users.error instanceof ApiRequestError ? users.error.message : "Could not load users."}
                  </AlertDescription>
                </Alert>
              )}
              <div className="overflow-x-auto rounded-md border">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Name</TableHead>
                      <TableHead>Email</TableHead>
                      <TableHead>Role</TableHead>
                      <TableHead>Last sign-in</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead className="text-right">Action</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {(users.data ?? []).map((u) => (
                      <TableRow key={u.id}>
                        <TableCell className="font-medium">{u.name}</TableCell>
                        <TableCell className="text-muted-foreground">{u.email}</TableCell>
                        <TableCell>{ROLE_LABEL[u.role]}</TableCell>
                        <TableCell className="tabular-nums text-muted-foreground">{formatWhen(u.lastLoginAt)}</TableCell>
                        <TableCell>
                          <Badge variant={u.active ? "secondary" : "outline"}>{u.active ? "Active" : "Inactive"}</Badge>
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap">
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={u.id === session?.userId}
                            onClick={() => setResetFor({ id: u.id, name: u.name })}
                          >
                            Reset password
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={u.id === session?.userId || toggleActive.isPending}
                            onClick={() => toggleActive.mutate({ id: u.id, active: !u.active })}
                          >
                            {u.active ? "Deactivate" : "Activate"}
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                    {users.isSuccess && users.data.length === 0 && (
                      <TableRow>
                        <TableCell colSpan={6} className="py-6 text-center text-muted-foreground">
                          No users yet.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Add user</CardTitle>
              <CardDescription>Share the password out of band; users can change it from the key icon next to their name.</CardDescription>
            </CardHeader>
            <CardContent>
              <form onSubmit={submit} className="grid gap-4 md:grid-cols-2">
                {formError && (
                  <Alert variant="destructive" className="md:col-span-2">
                    <AlertDescription>{formError}</AlertDescription>
                  </Alert>
                )}
                <div className="flex flex-col gap-1.5">
                  <Label htmlFor="nu-name">Full name</Label>
                  <Input id="nu-name" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} required />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label htmlFor="nu-email">Email</Label>
                  <Input
                    id="nu-email"
                    type="email"
                    value={form.email}
                    onChange={(e) => setForm({ ...form, email: e.target.value })}
                    required
                  />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label htmlFor="nu-role">Role</Label>
                  <select
                    id="nu-role"
                    className="h-8 rounded-lg border border-input bg-background px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                    value={form.role}
                    onChange={(e) => setForm({ ...form, role: e.target.value as Role })}
                  >
                    {ROLES.map((r) => (
                      <option key={r} value={r}>
                        {ROLE_LABEL[r]}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="flex flex-col gap-1.5">
                  <Label htmlFor="nu-password">Temporary password</Label>
                  <Input
                    id="nu-password"
                    type="password"
                    minLength={8}
                    value={form.password}
                    onChange={(e) => setForm({ ...form, password: e.target.value })}
                    required
                  />
                </div>
                <div className="md:col-span-2">
                  <Button type="submit" disabled={createUser.isPending}>
                    {createUser.isPending ? "Adding…" : "Add user"}
                  </Button>
                </div>
              </form>
            </CardContent>
          </Card>

          <ImportCard />

          <ChecklistTemplateCard />

          <OrganisationCard />

          <ReminderScheduleCard />

          <MailSettingsCard />

          <EmailTemplatesCard />

          <AuditCard />

          <ResetPasswordDialog open={resetFor !== null} onOpenChange={(o) => !o && setResetFor(null)} user={resetFor} />
        </div>

        <div className="flex min-w-0 flex-col gap-5">
          <Card>
            <CardHeader>
              <CardTitle>System</CardTitle>
            </CardHeader>
            <CardContent>
              <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-[13px]">
                <dt className="text-muted-foreground">Organisation</dt>
                <dd>{system.data?.orgName ?? "—"}</dd>
                <dt className="text-muted-foreground">Timezone</dt>
                <dd>{system.data?.timezone ?? "—"}</dd>
                <dt className="text-muted-foreground">Expiring soon</dt>
                <dd className="tabular-nums">{system.data ? `≤ ${system.data.thresholds.expiringSoonDays} days` : "—"}</dd>
                <dt className="text-muted-foreground">Urgent</dt>
                <dd className="tabular-nums">{system.data ? `≤ ${system.data.thresholds.urgentDays} days` : "—"}</dd>
                <dt className="text-muted-foreground">Server</dt>
                <dd className="font-mono text-[12px]">{health?.version ? `v${health.version}` : "—"}</dd>
              </dl>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Automation</CardTitle>
              <CardDescription>The scheduler runs the daily expiry sweep and sends queued emails.</CardDescription>
            </CardHeader>
            <CardContent>
              {scheduler ? (
                <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-[13px]">
                  <dt className="text-muted-foreground">Status</dt>
                  <dd>
                    <Badge variant={scheduler.stale ? "destructive" : "secondary"}>{scheduler.stale ? "Stale" : "Running"}</Badge>
                  </dd>
                  <dt className="text-muted-foreground">Host</dt>
                  <dd className="font-mono text-[12px]">{scheduler.hostname}</dd>
                  <dt className="text-muted-foreground">Last heartbeat</dt>
                  <dd className="tabular-nums">{formatWhen(scheduler.lastHeartbeatAt)}</dd>
                  <dt className="text-muted-foreground">Last sweep</dt>
                  <dd className="tabular-nums">{formatWhen(scheduler.lastSweepAt)}</dd>
                </dl>
              ) : (
                <p className="text-[13px] text-muted-foreground">
                  {system.isPending ? "Loading…" : "No scheduler has reported yet. Start the server with SCHEDULER=on."}
                </p>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </>
  );
}
