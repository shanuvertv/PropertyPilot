import { useState } from "react";
import { Bell, KeyRound, LogOut, Menu, Search, WifiOff } from "lucide-react";
import { NavLink, Outlet, useLocation } from "react-router";

import { ROLE_LABEL } from "@/api/types";
import { CommandPalette } from "@/components/CommandPalette";
import { ChangePasswordDialog } from "@/components/PasswordDialogs";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent } from "@/components/ui/sheet";
import { useApp } from "@/lib/app-state";
import { APP_NAME } from "@/lib/format";
import { useLiveEvents } from "@/lib/live";
import { NAV, phoneTabs } from "@/lib/nav";
import { cn } from "@/lib/utils";

function openSearch() {
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true }));
}

function UnreadBadge({ count, className }: { count: number; className?: string }) {
  if (count <= 0) return null;
  return (
    <span
      className={cn(
        "rounded-full bg-primary px-1.5 text-[10.5px] font-semibold text-primary-foreground tabular-nums",
        className,
      )}
      aria-label={`${count} unread`}
    >
      {count > 99 ? "99+" : count}
    </span>
  );
}

/**
 * Application frame. Desktop (≥ md): the 12-item sidebar from spec §20.
 * Phone (< md, i.e. the Android build): top bar + bottom tabs for the four most-used
 * modules of the signed-in role, with the full module list in a "More" sheet.
 */
export function Shell() {
  const { session, can, signOut, health, serverUrl } = useApp();
  const [passwordOpen, setPasswordOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const unread = useLiveEvents();
  const location = useLocation();
  if (!session) return null;

  const items = NAV.filter((item) => can(item.requires));
  const tabs = phoneTabs(items);
  const inMore = !tabs.some((t) => (t.to === "/" ? location.pathname === "/" : location.pathname.startsWith(t.to)));
  const disconnected = !health?.ok;

  return (
    <div className="flex h-dvh overflow-hidden bg-background text-foreground">
      {/* ---- desktop sidebar */}
      <aside className="hidden w-60 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground md:flex">
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
            onClick={openSearch}
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
                  {item.to === "/notifications" && <UnreadBadge count={unread} className="ml-auto" />}
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
        {/* ---- phone top bar */}
        <header className="flex items-center gap-1 border-b bg-sidebar px-3 pt-[env(safe-area-inset-top)] md:hidden">
          <div className="flex h-12 min-w-0 flex-1 items-center">
            <span className="truncate text-[15px] font-semibold tracking-tight">{APP_NAME}</span>
          </div>
          <Button variant="ghost" size="icon" onClick={openSearch} aria-label="Search">
            <Search />
          </Button>
          {can("VIEW_DASHBOARD") && (
            <NavLink
              to="/notifications"
              className="relative inline-flex size-9 items-center justify-center rounded-lg hover:bg-sidebar-accent"
              aria-label="Notifications"
            >
              <Bell className="size-4" aria-hidden="true" />
              <UnreadBadge count={unread} className="absolute -top-0.5 -right-0.5" />
            </NavLink>
          )}
        </header>

        {disconnected && (
          <div
            role="status"
            className="flex items-center gap-2 border-b border-destructive/30 bg-destructive/10 px-4 py-2 text-[13px] text-destructive md:px-5"
          >
            <WifiOff className="size-4 shrink-0" aria-hidden="true" />
            <span>Disconnected from the server — retrying. Changes cannot be saved until the connection is back.</span>
          </div>
        )}
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[1200px] px-4 py-4 pb-24 md:px-8 md:py-7 md:pb-7">
            <Outlet />
          </div>
        </main>

        {/* ---- phone bottom tabs */}
        <nav
          className="flex shrink-0 items-stretch border-t bg-sidebar pb-[env(safe-area-inset-bottom)] md:hidden"
          aria-label="Main"
        >
          {tabs.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === "/"}
              className={({ isActive }) =>
                cn(
                  "flex min-w-0 flex-1 flex-col items-center justify-center gap-0.5 py-2 text-[10.5px] text-muted-foreground",
                  isActive && "font-medium text-sidebar-primary",
                )
              }
            >
              <item.icon className="size-5" aria-hidden="true" />
              <span className="max-w-full truncate">{item.short ?? item.label}</span>
            </NavLink>
          ))}
          <button
            type="button"
            onClick={() => setMoreOpen(true)}
            className={cn(
              "flex min-w-0 flex-1 flex-col items-center justify-center gap-0.5 py-2 text-[10.5px] text-muted-foreground",
              inMore && "font-medium text-sidebar-primary",
            )}
            aria-haspopup="dialog"
            aria-expanded={moreOpen}
          >
            <Menu className="size-5" aria-hidden="true" />
            <span>More</span>
          </button>
        </nav>
      </div>

      {/* ---- phone "More" sheet: every module + account actions */}
      <Sheet open={moreOpen} onOpenChange={setMoreOpen}>
        <SheetContent title="Menu">
          <ul className="flex flex-col">
            {items.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  end={item.to === "/"}
                  onClick={() => setMoreOpen(false)}
                  className={({ isActive }) =>
                    cn(
                      "flex min-h-12 items-center gap-3 rounded-lg px-3 text-[14.5px]",
                      isActive ? "bg-sidebar-accent font-medium text-sidebar-primary" : "hover:bg-sidebar-accent",
                    )
                  }
                >
                  <item.icon className="size-5 shrink-0" aria-hidden="true" />
                  <span className="flex-1 truncate">{item.label}</span>
                  {item.to === "/notifications" && <UnreadBadge count={unread} />}
                </NavLink>
              </li>
            ))}
          </ul>
          <div className="mt-2 border-t px-3 pt-3">
            <div className="text-[13.5px] font-medium">{session.name}</div>
            <div className="text-[12px] text-muted-foreground">
              {ROLE_LABEL[session.role]} · {serverUrl.replace(/^https?:\/\//, "")}
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              <Button
                variant="outline"
                onClick={() => {
                  setMoreOpen(false);
                  setPasswordOpen(true);
                }}
              >
                <KeyRound />
                Change password
              </Button>
              <Button variant="outline" onClick={() => void signOut()}>
                <LogOut />
                Sign out
              </Button>
            </div>
          </div>
        </SheetContent>
      </Sheet>
    </div>
  );
}
