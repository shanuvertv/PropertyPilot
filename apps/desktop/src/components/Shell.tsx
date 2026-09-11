import { useState } from "react";
import { KeyRound, LogOut, Search, WifiOff } from "lucide-react";
import { NavLink, Outlet } from "react-router";

import { ROLE_LABEL } from "@/api/types";
import { CommandPalette } from "@/components/CommandPalette";
import { ChangePasswordDialog } from "@/components/PasswordDialogs";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { APP_NAME } from "@/lib/format";
import { useLiveEvents } from "@/lib/live";
import { NAV } from "@/lib/nav";
import { cn } from "@/lib/utils";

export function Shell() {
  const { session, can, signOut, health, serverUrl } = useApp();
  const [passwordOpen, setPasswordOpen] = useState(false);
  if (!session) return null;

  const items = NAV.filter((item) => can(item.requires));
  const disconnected = !health?.ok;
  const unread = useLiveEvents();

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <aside className="flex w-60 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
        <div className="px-5 pt-5 pb-4">
          <div className="text-[15px] font-semibold tracking-tight">{APP_NAME}</div>
          <div className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground" title={serverUrl}>
            {serverUrl.replace(/^https?:\/\//, "")}
          </div>
        </div>
        <div className="px-3 pb-2">
          <button
            type="button"
            className="flex w-full items-center gap-2 rounded-md border bg-background px-2.5 py-1.5 text-[12.5px] text-muted-foreground hover:text-foreground"
            onClick={() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true }))}
          >
            <Search className="size-3.5" aria-hidden="true" />
            <span className="flex-1 text-left">Search…</span>
            <kbd className="rounded border px-1 text-[10px]">Ctrl K</kbd>
          </button>
        </div>
        <nav className="flex-1 overflow-y-auto px-3" aria-label="Main">
          <ul className="flex flex-col gap-0.5">
            {items.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  end={item.to === "/"}
                  className={({ isActive }) =>
                    cn(
                      "flex items-center gap-2.5 rounded-md px-2.5 py-2 text-[13.5px] transition-colors",
                      "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
                      isActive && "bg-sidebar-accent font-medium text-sidebar-primary",
                    )
                  }
                >
                  <item.icon className="size-4 shrink-0" aria-hidden="true" />
                  <span className="truncate">{item.label}</span>
                  {item.to === "/notifications" && unread > 0 && (
                    <span className="ml-auto rounded-full bg-primary px-1.5 text-[10.5px] font-semibold text-primary-foreground tabular-nums" aria-label={`${unread} unread`}>
                      {unread > 99 ? "99+" : unread}
                    </span>
                  )}
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>
        <div className="border-t border-sidebar-border px-4 py-3">
          <div className="flex items-center justify-between gap-2">
            <div className="min-w-0">
              <div className="truncate text-[13px] font-medium">{session.name}</div>
              <div className="truncate text-[11.5px] text-muted-foreground">{ROLE_LABEL[session.role]}</div>
            </div>
            <div className="flex shrink-0 items-center">
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => setPasswordOpen(true)}
                title="Change password"
                aria-label="Change password"
              >
                <KeyRound />
              </Button>
              <Button variant="ghost" size="icon-sm" onClick={() => void signOut()} title="Sign out" aria-label="Sign out">
                <LogOut />
              </Button>
            </div>
          </div>
        </div>
      </aside>
      <ChangePasswordDialog open={passwordOpen} onOpenChange={setPasswordOpen} />

      <CommandPalette />
      <div className="flex min-w-0 flex-1 flex-col">
        {disconnected && (
          <div
            role="status"
            className="flex items-center gap-2 border-b border-destructive/30 bg-destructive/10 px-5 py-2 text-[13px] text-destructive"
          >
            <WifiOff className="size-4" aria-hidden="true" />
            Disconnected from the server — retrying. Changes cannot be saved until the connection is back.
          </div>
        )}
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[1200px] px-8 py-7">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
