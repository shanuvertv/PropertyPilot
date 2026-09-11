import { useQuery } from "@tanstack/react-query";

import { useApp } from "@/lib/app-state";

/** Reference data used by forms and filters; cached for a minute. */
export function useEmployees() {
  const { api } = useApp();
  return useQuery({ queryKey: ["employees"], queryFn: () => api.employees(), staleTime: 60_000 });
}

export function useBuildingOptions() {
  const { api } = useApp();
  return useQuery({ queryKey: ["buildings", "options"], queryFn: () => api.buildingOptions(), staleTime: 60_000 });
}

export function useTenantOptions() {
  const { api } = useApp();
  return useQuery({ queryKey: ["tenants", "options"], queryFn: () => api.tenantOptions(), staleTime: 60_000 });
}
