import { Navigate, Route, Routes } from "react-router";

import { Shell } from "@/components/Shell";
import { useApp } from "@/lib/app-state";
import { NAV } from "@/lib/nav";
import { BootstrapPage } from "@/pages/Bootstrap";
import { BuildingDetailPage } from "@/pages/buildings/BuildingDetailPage";
import { BuildingsPage } from "@/pages/buildings/BuildingsPage";
import { ContractDetailPage } from "@/pages/contracts/ContractDetailPage";
import { ContractsPage } from "@/pages/contracts/ContractsPage";
import { DashboardPage } from "@/pages/Dashboard";
import { EmailsPage } from "@/pages/EmailsPage";
import { NoticesPage } from "@/pages/NoticesPage";
import { NotificationsPage } from "@/pages/NotificationsPage";
import { FollowUpsPage } from "@/pages/FollowUpsPage";
import { LoginPage } from "@/pages/Login";
import { ReportsPage } from "@/pages/ReportsPage";
import { RenewalCasePage } from "@/pages/renewals/RenewalCasePage";
import { RenewalsPage } from "@/pages/renewals/RenewalsPage";
import { SettingsPage } from "@/pages/Settings";
import { SetupPage } from "@/pages/Setup";
import { TenantDetailPage } from "@/pages/tenants/TenantDetailPage";
import { TenantsPage } from "@/pages/tenants/TenantsPage";
import { UnitsPage } from "@/pages/units/UnitsPage";

function Loading() {
  return (
    <div className="flex min-h-screen items-center justify-center text-[13px] text-muted-foreground" role="status">
      Connecting…
    </div>
  );
}

/** Route guard: a screen the role may not open bounces to the dashboard. */
function Guarded({ path, children }: { path: string; children: React.ReactNode }) {
  const { can } = useApp();
  const item = NAV.find((n) => n.to === path);
  if (item && !can(item.requires)) return <Navigate to="/" replace />;
  return <>{children}</>;
}

export default function App() {
  const { stage } = useApp();

  switch (stage) {
    case "loading":
      return <Loading />;
    case "setup":
      return <SetupPage />;
    case "bootstrap":
      return <BootstrapPage />;
    case "login":
      return <LoginPage />;
    case "ready":
      return (
        <Routes>
          <Route element={<Shell />}>
            <Route index element={<DashboardPage />} />
            <Route path="/buildings" element={<Guarded path="/buildings"><BuildingsPage /></Guarded>} />
            <Route path="/buildings/:id" element={<Guarded path="/buildings"><BuildingDetailPage /></Guarded>} />
            <Route path="/units" element={<Guarded path="/units"><UnitsPage /></Guarded>} />
            <Route path="/tenants" element={<Guarded path="/tenants"><TenantsPage /></Guarded>} />
            <Route path="/tenants/:id" element={<Guarded path="/tenants"><TenantDetailPage /></Guarded>} />
            <Route path="/contracts" element={<Guarded path="/contracts"><ContractsPage /></Guarded>} />
            <Route path="/contracts/:id" element={<Guarded path="/contracts"><ContractDetailPage /></Guarded>} />
            <Route path="/renewals" element={<Guarded path="/renewals"><RenewalsPage /></Guarded>} />
            <Route path="/renewals/:id" element={<Guarded path="/renewals"><RenewalCasePage /></Guarded>} />
            <Route path="/follow-ups" element={<Guarded path="/follow-ups"><FollowUpsPage /></Guarded>} />
            <Route path="/notices" element={<Guarded path="/notices"><NoticesPage /></Guarded>} />
            <Route path="/emails" element={<Guarded path="/emails"><EmailsPage /></Guarded>} />
            <Route path="/notifications" element={<Guarded path="/notifications"><NotificationsPage /></Guarded>} />
            <Route path="/reports" element={<Guarded path="/reports"><ReportsPage /></Guarded>} />
            <Route path="/settings" element={<Guarded path="/settings"><SettingsPage /></Guarded>} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      );
  }
}
