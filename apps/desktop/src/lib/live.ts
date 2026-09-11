import { useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { useApp } from "@/lib/app-state";

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Native Windows/Android toast via the Tauri notification plugin; no-op in a plain browser. */
export async function nativeToast(title: string, body?: string) {
  if (!inTauri()) return;
  try {
    const mod = await import("@tauri-apps/plugin-notification");
    let granted = await mod.isPermissionGranted();
    if (!granted) granted = (await mod.requestPermission()) === "granted";
    if (granted) mod.sendNotification({ title, body });
  } catch {
    // plugin missing or denied: the in-app bell still shows it
  }
}

/**
 * Keeps the app live: subscribes to the server's SSE stream (PLAN.md D11) and
 * invalidates caches when the server says something changed. Falls back to
 * polling the unread count every minute if the stream cannot connect.
 */
export function useLiveEvents() {
  const { api, session } = useApp();
  const queryClient = useQueryClient();
  const unread = useQuery({
    queryKey: ["notifications", "count"],
    queryFn: () => api.unreadCount(),
    enabled: !!session,
    refetchInterval: 60_000,
  });
  const lastUnread = useRef<number | null>(null);

  // Toast when new unread notifications arrive.
  useEffect(() => {
    const n = unread.data?.unread;
    if (n === undefined) return;
    if (lastUnread.current !== null && n > lastUnread.current) {
      api
        .notifications(true)
        .then((list) => {
          const newest = list[0];
          if (newest) void nativeToast(newest.title, newest.body ?? undefined);
        })
        .catch(() => {});
    }
    lastUnread.current = n;
  }, [unread.data?.unread, api]);

  useEffect(() => {
    if (!session) return;
    const url = api.eventsUrl();
    if (!url || typeof EventSource === "undefined") return;
    const es = new EventSource(url);
    const onNotifications = () => void queryClient.invalidateQueries({ queryKey: ["notifications"] });
    const onData = () => {
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      void queryClient.invalidateQueries({ queryKey: ["contracts"] });
      void queryClient.invalidateQueries({ queryKey: ["renewals"] });
      void queryClient.invalidateQueries({ queryKey: ["units"] });
      void queryClient.invalidateQueries({ queryKey: ["follow-ups"] });
      void queryClient.invalidateQueries({ queryKey: ["emails"] });
      void queryClient.invalidateQueries({ queryKey: ["system-status"] });
    };
    es.addEventListener("notifications", onNotifications);
    es.addEventListener("data", onData);
    return () => es.close();
  }, [api, session, queryClient]);

  return unread.data?.unread ?? 0;
}
