//! User roles and the permission matrix (spec §18, PLAN.md §6.7).

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::error::PermissionDenied;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Role {
    Admin,
    Leasing,
    Operations,
    Management,
}

/// Every distinct thing a user can do. One capability per row of the matrix;
/// "view" rights are separate from "manage" rights so read-only roles work.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Capability {
    ViewDashboard,
    ViewBuildings,
    ManageBuildings,
    ViewUnits,
    ManageUnits,
    UpdateUnitStatus,
    ViewTenants,
    ManageTenants,
    ViewContracts,
    ManageContracts,
    ViewRenewals,
    ManageRenewals,
    SendNotices,
    ViewFollowUps,
    ManageFollowUps,
    ViewReports,
    ManageSettings,
    ManageUsers,
    ViewAuditTrail,
    /// People living in a unit (shared accommodation): view / add, edit, move out.
    ViewOccupants,
    ManageOccupants,
    /// Bills and costs per unit, split between its occupants.
    ViewExpenses,
    ManageExpenses,
}

impl Role {
    /// The permission matrix from PLAN.md §6.7, expressed as data.
    pub fn allows(self, cap: Capability) -> bool {
        use Capability::*;
        match self {
            Role::Admin => true,
            Role::Leasing => !matches!(cap, ManageSettings | ManageUsers | ViewAuditTrail),
            Role::Operations => matches!(
                cap,
                ViewUnits
                    | UpdateUnitStatus
                    | ViewTenants
                    | ViewBuildings
                    | ViewOccupants
                    | ManageOccupants
                    | ViewExpenses
            ),
            Role::Management => matches!(
                cap,
                ViewDashboard
                    | ViewBuildings
                    | ViewUnits
                    | ViewTenants
                    | ViewContracts
                    | ViewRenewals
                    | ViewFollowUps
                    | ViewReports
                    | ViewAuditTrail
                    | ViewOccupants
                    | ViewExpenses
            ),
        }
    }

    /// Guard for service-layer entry points: `role.require(Capability::ManageContracts)?`.
    pub fn require(self, cap: Capability) -> Result<(), PermissionDenied> {
        if self.allows(cap) {
            Ok(())
        } else {
            Err(PermissionDenied {
                role: self,
                capability: cap,
            })
        }
    }

    /// Whether the sidebar/settings area should be visible at all for this role.
    pub fn can_open_settings(self) -> bool {
        self.allows(Capability::ManageSettings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn admin_can_do_everything() {
        for cap in Capability::iter() {
            assert!(Role::Admin.allows(cap), "admin should allow {cap}");
        }
    }

    #[test]
    fn leasing_matches_spec_section_18() {
        let r = Role::Leasing;
        assert!(r.allows(Capability::ManageTenants));
        assert!(r.allows(Capability::ManageUnits));
        assert!(r.allows(Capability::ManageContracts));
        assert!(r.allows(Capability::ManageRenewals));
        assert!(r.allows(Capability::SendNotices));
        assert!(r.allows(Capability::ManageFollowUps));
        assert!(!r.allows(Capability::ManageSettings));
        assert!(!r.allows(Capability::ManageUsers));
    }

    #[test]
    fn operations_is_units_and_tenant_view_only() {
        let r = Role::Operations;
        assert!(r.allows(Capability::ViewUnits));
        assert!(r.allows(Capability::UpdateUnitStatus));
        assert!(r.allows(Capability::ViewTenants));
        assert!(!r.allows(Capability::ManageUnits));
        assert!(!r.allows(Capability::ViewContracts));
        assert!(!r.allows(Capability::SendNotices));
        assert!(!r.allows(Capability::ViewReports));
    }

    #[test]
    fn management_is_read_only() {
        let r = Role::Management;
        assert!(r.allows(Capability::ViewDashboard));
        assert!(r.allows(Capability::ViewReports));
        assert!(r.allows(Capability::ViewRenewals));
        for cap in Capability::iter() {
            let name = cap.to_string();
            if name.starts_with("MANAGE") || name.starts_with("SEND") || name.starts_with("UPDATE")
            {
                assert!(!r.allows(cap), "management must not {cap}");
            }
        }
    }

    #[test]
    fn require_returns_typed_error() {
        let err = Role::Management
            .require(Capability::SendNotices)
            .unwrap_err();
        assert_eq!(err.role, Role::Management);
        assert_eq!(err.capability, Capability::SendNotices);
        assert_eq!(err.to_string(), "MANAGEMENT is not allowed to SEND_NOTICES");
    }

    #[test]
    fn role_round_trips_through_strings_and_serde() {
        for role in Role::iter() {
            let s = role.to_string();
            assert_eq!(s.parse::<Role>().unwrap(), role);
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, format!("\"{s}\""));
            assert_eq!(serde_json::from_str::<Role>(&json).unwrap(), role);
        }
    }
}
