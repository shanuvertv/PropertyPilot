import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { HashRouter } from "react-router";

import App from "@/App";
import { ApiRequestError } from "@/api/client";
import { AppProvider } from "@/lib/app-state";

import "@/index.css";

// Hash routing: the packaged app is served from a custom protocol where deep URLs
// cannot be resolved server-side, and it behaves identically on Android.
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // One retry for flaky networks; a 4xx (not found, forbidden, bad id) will not
      // get better on a retry, so surface it immediately instead of showing "Loading".
      retry: (count, err) => count < 1 && !(err instanceof ApiRequestError && err.status >= 400 && err.status < 500),
      refetchOnWindowFocus: false,
      staleTime: 10_000,
    },
  },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <HashRouter>
        <AppProvider>
          <App />
        </AppProvider>
      </HashRouter>
    </QueryClientProvider>
  </React.StrictMode>,
);
