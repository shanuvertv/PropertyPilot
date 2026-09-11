import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiClient, ApiRequestError, normalizeBaseUrl } from "@/api/client";
import type { Capability, HealthResponse, SessionInfo } from "@/api/types";
import { SECURE_KEYS, secureDelete, secureGet, secureSet } from "@/lib/secure";

export type Stage = "loading" | "setup" | "bootstrap" | "login" | "ready";

interface AppState {
  stage: Stage;
  serverUrl: string;
  health: HealthResponse | undefined;
  healthError: string | null;
  session: SessionInfo | null;
  api: ApiClient;
  can: (cap: Capability) => boolean;
  configureServer: (url: string) => Promise<void>;
  signIn: (email: string, password: string) => Promise<void>;
  bootstrap: (name: string, email: string, password: string) => Promise<void>;
  signOut: () => Promise<void>;
  /** Forget the saved server (and token) and return to the Setup screen. */
  changeServer: () => Promise<void>;
  refreshHealth: () => void;
}

const Ctx = createContext<AppState | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const [loaded, setLoaded] = useState(false);
  const [serverUrl, setServerUrl] = useState("");
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [sessionChecked, setSessionChecked] = useState(false);
  const tokenRef = useRef<string | null>(null);

  // One client per server URL; it reads the token lazily so a login never rebuilds it.
  const api = useMemo(
    () =>
      new ApiClient({
        baseUrl: serverUrl,
        getToken: () => tokenRef.current,
        onUnauthorized: () => {
          tokenRef.current = null;
          void secureDelete(SECURE_KEYS.token);
          setSession(null);
        },
      }),
    [serverUrl],
  );

  // Load persisted server URL + token once.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const [url, token] = await Promise.all([secureGet(SECURE_KEYS.serverUrl), secureGet(SECURE_KEYS.token)]);
      if (cancelled) return;
      tokenRef.current = token;
      setServerUrl(url ?? "");
      setLoaded(true);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const healthQuery = useQuery({
    queryKey: ["health", serverUrl],
    queryFn: () => api.health(),
    enabled: loaded && serverUrl !== "",
    retry: false,
    refetchInterval: (q) => (q.state.data?.ok ? 30_000 : 5_000),
  });

  // Restore the session from a stored token once the server answers.
  useEffect(() => {
    if (!loaded || !healthQuery.data?.ok) return;
    if (sessionChecked) return;
    if (!tokenRef.current) {
      setSessionChecked(true);
      return;
    }
    let cancelled = false;
    api
      .me()
      .then((s) => {
        if (!cancelled) setSession(s);
      })
      .catch(() => {
        // token expired or revoked: fall through to login
      })
      .finally(() => {
        if (!cancelled) setSessionChecked(true);
      });
    return () => {
      cancelled = true;
    };
  }, [loaded, healthQuery.data?.ok, sessionChecked, api]);

  const configureServer = useCallback(
    async (url: string) => {
      const normalized = normalizeBaseUrl(url);
      const probe = new ApiClient({ baseUrl: normalized, getToken: () => null });
      await probe.health(); // throws ApiRequestError("NETWORK") when unreachable
      await secureSet(SECURE_KEYS.serverUrl, normalized);
      setSessionChecked(false);
      setServerUrl(normalized);
    },
    [],
  );

  const acceptLogin = useCallback(
    async (token: string, s: SessionInfo) => {
      tokenRef.current = token;
      await secureSet(SECURE_KEYS.token, token);
      setSession(s);
      setSessionChecked(true);
      await queryClient.invalidateQueries({ queryKey: ["health"] });
    },
    [queryClient],
  );

  const signIn = useCallback(
    async (email: string, password: string) => {
      const res = await api.login({ email, password });
      await acceptLogin(res.token, res.session);
    },
    [api, acceptLogin],
  );

  const bootstrap = useCallback(
    async (name: string, email: string, password: string) => {
      const res = await api.bootstrap({ name, email, password });
      await acceptLogin(res.token, res.session);
    },
    [api, acceptLogin],
  );

  const signOut = useCallback(async () => {
    try {
      await api.logout();
    } catch {
      // already signed out server-side; clear locally regardless
    }
    tokenRef.current = null;
    await secureDelete(SECURE_KEYS.token);
    setSession(null);
    queryClient.clear();
  }, [api, queryClient]);

  const changeServer = useCallback(async () => {
    tokenRef.current = null;
    await Promise.all([secureDelete(SECURE_KEYS.token), secureDelete(SECURE_KEYS.serverUrl)]);
    setSession(null);
    setSessionChecked(false);
    setServerUrl("");
    queryClient.clear();
  }, [queryClient]);

  const health = healthQuery.data;
  const healthError =
    healthQuery.error instanceof ApiRequestError
      ? healthQuery.error.message
      : healthQuery.error
        ? String(healthQuery.error)
        : health && !health.ok
          ? "The server is reachable but its database is unavailable. Ask your administrator to check the server."
          : null;

  // A signed-in user stays in the app through connection blips (the Shell shows a banner);
  // everyone else is routed by what the server reports.
  let stage: Stage = "loading";
  if (loaded) {
    if (session) stage = "ready";
    else if (!serverUrl || healthQuery.isError) stage = "setup";
    else if (!health) stage = "loading";
    else if (!health.ok) stage = "setup";
    else if (!health.usersExist) stage = "bootstrap";
    else if (!sessionChecked) stage = "loading";
    else stage = "login";
  }

  const can = useCallback((cap: Capability) => session?.capabilities.includes(cap) ?? false, [session]);

  const value: AppState = {
    stage,
    serverUrl,
    health,
    healthError,
    session,
    api,
    can,
    configureServer,
    signIn,
    bootstrap,
    signOut,
    changeServer,
    refreshHealth: () => void healthQuery.refetch(),
  };

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useApp(): AppState {
  const v = useContext(Ctx);
  if (!v) throw new Error("useApp must be used inside <AppProvider>");
  return v;
}
