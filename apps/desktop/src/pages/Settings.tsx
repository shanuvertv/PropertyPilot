import { useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiRequestError } from "@/api/client";
import { ROLE_LABEL, ROLES, type Role, type UserSummary } from "@/api/types";
import { DataTable, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useApp } from "@/lib/app-state";
import { ChecklistTemplateCard } from "@/pages/settings/ChecklistTemplateCard";
import { EmailTemplatesCard } from "@/pages/settings/EmailTemplatesCard";
import { OrganisationCard, ReminderScheduleCard } from "@/pages/settings/AutomationCards";
import { AuditCard } from "@/pages/settings/AuditCard";
import { ImportCard } from "@/pages/settings/ImportCard";
import { MailSettingsCard } from "@/pages/settings/MailSettingsCard";
import { ResetPasswordDialog } from "@/components/PasswordDialogs";
import { FormDialog, selectClass } from "@/components/forms";
import { SearchBox } from "@/components/SearchBox";
import { useLocalFilter } from "@/lib/local-filter";

function formatWhen(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

export function SettingsPage() {
  const { api, health, session } = useApp();
  const queryClient = useQueryClient();

  const users = useQuery({ queryKey: ["users"], queryFn: () => api.listUsers() });
  const userFilter = useLocalFilter(users.data, (u) => [u.name, u.email, ROLE_LABEL[u.role], u.active ? "active" : "inactive"]);
  const [roleFilter, setRoleFilter] = useState("");
  const system = useQuery({ queryKey: ["system-status"], queryFn: () => api.systemStatus(), refetchInterval: 30_000 });

  const [form, setForm] = useState({ name: "", email: "", role: "LEASING" as Role, password: "" });
  const [resetFor, setResetFor] = useState<{ id: string; name: string } | null>(null);
  const [deleteFor, setDeleteFor] = useState<{ id: string; name: string; email: string } | null>(null);
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
  const [view, setView] = useViewMode("users");

  const userCols: Column<UserSummary>[] = [
    { key: "name", header: "Name", card: "title", render: (u) => <span className="font-medium">{u.name}</span> },
    { key: "email", header: "Email", card: "subtitle", render: (u) => <span className="text-muted-foreground">{u.email}</span> },
    { key: "role", header: "Role", card: "metric", render: (u) => ROLE_LABEL[u.role] },
    { key: "login", header: "Last sign-in", card: "metric", render: (u) => <span className="text-muted-foreground tabular-nums">{formatWhen(u.lastLoginAt)}</span> },
    { key: "status", header: "Status", card: "badge", render: (u) => <Badge variant={u.active ? "secondary" : "outline"}>{u.active ? "Active" : "Inactive"}</Badge> },
    {
      key: "actions",
      header: "",
      className: "text-right whitespace-nowrap",
      render: (u) => (
        <span className="inline-flex flex-wrap justify-end gap-1">
          <Button variant="ghost" size="sm" disabled={u.id === session?.userId} onClick={() => setResetFor({ id: u.id, name: u.name })}>
            Reset password
          </Button>
          <Button variant="ghost" size="sm" disabled={u.id === session?.userId || toggleActive.isPending} onClick={() => toggleActive.mutate({ id: u.id, active: !u.active })}>
            {u.active ? "Deactivate" : "Activate"}
          </Button>
          {!u.active && u.id !== session?.userId && (
            <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive" onClick={() => setDeleteFor({ id: u.id, name: u.name, email: u.email })}>
              Delete
            </Button>
          )}
        </span>
      ),
    },
  ];

  return (
    <>
      <PageHeader title="Settings" description="Users and roles, tenant-list import, renewal checklist, email sending and templates, automation." />

      <div className="grid grid-cols-[minmax(0,1fr)] gap-5 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="flex min-w-0 flex-col gap-5">
          <Card>
            <CardHeader>
              <CardTitle>Users</CardTitle>
              <CardDescription>Deactivating a user signs them out everywhere immediately; an inactive user can then be deleted.</CardDescription>
            </CardHeader>
            <CardContent>
              {users.isError && (
                <Alert variant="destructive" className="mb-3">
                  <AlertDescription>
                    {users.error instanceof ApiRequestError ? users.error.message : "Could not load users."}
                  </AlertDescription>
                </Alert>
              )}
              <div className="mb-3 flex flex-wrap items-center gap-2">
                <SearchBox value={userFilter.q} onChange={userFilter.setQ} placeholder="Search name or email" />
                <select className={`${selectClass} w-auto`} value={roleFilter} onChange={(e) => setRoleFilter(e.target.value)} aria-label="Role">
                  <option value="">All roles</option>
                  {ROLES.map((r) => (
                    <option key={r} value={r}>{ROLE_LABEL[r]}</option>
                  ))}
                </select>
                <ViewToggle value={view} onChange={setView} className="ml-auto" />
              </div>
              <DataTable
                view={view}
                columns={userCols}
                rows={userFilter.filtered?.filter((u) => !roleFilter || u.role === roleFilter)}
                rowKey={(u) => u.id}
                loading={users.isPending}
                empty={userFilter.q || roleFilter ? "No user matches the filters." : "No users yet."}
              />
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
          <FormDialog
            open={deleteFor !== null}
            onOpenChange={(o) => !o && setDeleteFor(null)}
            title={deleteFor ? `Delete ${deleteFor.name}?` : "Delete user"}
            description={
              deleteFor
                ? `${deleteFor.email} will be signed out everywhere and removed from this list; the address can be used for a new account. Their name stays on the history of what they did.`
                : undefined
            }
            submitLabel="Delete user"
            destructive
            onSubmit={async () => {
              if (!deleteFor) return;
              await api.deleteUser(deleteFor.id);
              await queryClient.invalidateQueries({ queryKey: ["users"] });
            }}
          >
            <span className="sr-only">Confirm</span>
          </FormDialog>
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
