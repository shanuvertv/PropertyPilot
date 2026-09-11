import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { HashRouter } from "react-router";

import App from "@/App";
import { AppProvider } from "@/lib/app-state";

import "@/index.css";

// Hash routing: the packaged app is served from a custom protocol where deep URLs
// cannot be resolved server-side, and it behaves identically on Android.
const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, refetchOnWindowFocus: false, staleTime: 10_000 },
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
