import { Navigate, Route, Routes } from "react-router";

import { Shell } from "@/components/Shell";
import { useApp } from "@/lib/app-state";
import { homeFor, NAV } from "@/lib/nav";
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
import { UnitPage } from "@/pages/units/UnitPage";
import { ExpensesPage } from "@/pages/expenses/ExpensesPage";
import { ExpenseDetailPage } from "@/pages/expenses/ExpenseDetailPage";

function Loading() {
  return (
    <div className="flex min-h-screen items-center justify-center text-[13px] text-muted-foreground" role="status">
      Connecting…
    </div>
  );
}

/** Route guard: a screen the role may not open bounces to the home screen. */
function Guarded({ path, children }: { path: string; children: React.ReactNode }) {
  const { can } = useApp();
  const item = NAV.find((n) => n.to === path);
  if (item && !can(item.requires)) return <Navigate to="/" replace />;
  return <>{children}</>;
}

/**
 * Where "/" lands: the dashboard for roles that may see it, otherwise the first
 * module the role can use (Operations opens on Units instead of a forbidden dashboard).
 */
function Home() {
  const { can } = useApp();
  if (can("VIEW_DASHBOARD")) return <DashboardPage />;
  const first = homeFor(can);
  if (first) return <Navigate to={first.to} replace />;
  return (
    <p className="text-[13px] text-muted-foreground" role="status">
      Your role has no screens enabled yet. Ask an administrator to update your access.
    </p>
  );
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
            <Route index element={<Home />} />
            <Route path="/buildings" element={<Guarded path="/buildings"><BuildingsPage /></Guarded>} />
            <Route path="/buildings/:id" element={<Guarded path="/buildings"><BuildingDetailPage /></Guarded>} />
            <Route path="/units" element={<Guarded path="/units"><UnitsPage /></Guarded>} />
            <Route path="/units/:id" element={<Guarded path="/units"><UnitPage /></Guarded>} />
            <Route path="/expenses" element={<Guarded path="/expenses"><ExpensesPage /></Guarded>} />
            <Route path="/expenses/:id" element={<Guarded path="/expenses"><ExpenseDetailPage /></Guarded>} />
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
