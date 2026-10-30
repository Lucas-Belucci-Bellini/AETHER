use std::collections::BTreeSet;

use crate::tool::Permission;

/// Which permissions an agent holds. Starts empty: nothing is allowed until
/// granted. Grants come from application code only, never from model output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    granted: BTreeSet<Permission>,
}

impl Policy {
    /// A policy that grants nothing.
    pub fn deny_by_default() -> Self {
        Self::default()
    }

    /// Grants a permission.
    pub fn grant(mut self, permission: impl Into<Permission>) -> Self {
        self.granted.insert(permission.into());
        self
    }

    /// Revokes a permission if it is currently granted.
    ///
    /// Revoking an absent permission is a no-op. This keeps policy composition
    /// predictable and preserves deny-by-default behavior.
    pub fn revoke(mut self, permission: impl Into<Permission>) -> Self {
        self.granted.remove(&permission.into());
        self
    }

    /// `true` if `permission` was granted.
    pub fn is_granted(&self, permission: &Permission) -> bool {
        self.granted.contains(permission)
    }

    /// `Ok` if every required permission is granted, else the first missing one.
    pub fn check<'a>(&self, required: &'a [Permission]) -> Result<(), &'a Permission> {
        match required.iter().find(|p| !self.is_granted(p)) {
            Some(missing) => Err(missing),
            None => Ok(()),
        }
    }

    /// Granted permissions.
    pub fn granted(&self) -> impl Iterator<Item = &Permission> {
        self.granted.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_until_every_permission_is_granted() {
        let required = [
            Permission::new("files.read"),
            Permission::new("files.write"),
        ];
        let policy = Policy::deny_by_default();
        assert_eq!(policy.check(&required), Err(&required[0]));
        let policy = policy.grant("files.read");
        assert_eq!(policy.check(&required), Err(&required[1]));
        let policy = policy.grant("files.write");
        assert_eq!(policy.check(&required), Ok(()));
    }

    #[test]
    fn revoke_removes_only_the_requested_permission() {
        let policy = Policy::deny_by_default()
            .grant("files.read")
            .grant("files.write")
            .revoke("files.write");

        assert!(policy.is_granted(&Permission::new("files.read")));
        assert!(!policy.is_granted(&Permission::new("files.write")));
        assert_eq!(
            policy.granted().map(Permission::as_str).collect::<Vec<_>>(),
            ["files.read"]
        );
    }

    #[test]
    fn revoking_an_absent_permission_is_a_noop() {
        let policy = Policy::deny_by_default()
            .grant("files.read")
            .revoke("files.write");

        assert!(policy.is_granted(&Permission::new("files.read")));
        assert_eq!(policy.granted().count(), 1);
    }
}
