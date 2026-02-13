//! Minimal Landlock sandbox setup for Linux.
//!
//! This is a best-effort hardening layer: if Landlock is unavailable or setup
//! fails (e.g. older kernel, missing permissions), we log and continue without
//! sandboxing rather than crashing the service.

#[cfg(target_os = "linux")]
pub mod linux {
    use landlock::{
        path_beneath_rules, Access, AccessFs, ABI, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus,
    };
    use tracing::{info, warn};

    /// Initialize a minimal Landlock sandbox.
    ///
    /// Current policy:
    /// - Allow read-only access to `/app` (code, migrations, static files)
    /// - Deny write access outside `/app`
    pub fn init() {
        // The Landlock ABI should be incremented (and tested) regularly.
        let abi = ABI::V1;
        let access_all = AccessFs::from_all(abi);
        let access_read = AccessFs::from_read(abi);

        let ruleset = Ruleset::default();
        let result = ruleset
            .handle_access(access_all)
            .and_then(|r| r.create())
            .and_then(|r| r.add_rules(path_beneath_rules(&["/app"], access_read)))
            .and_then(|r| r.restrict_self());

        match result {
            Ok(status) => {
                match status.ruleset {
                    RulesetStatus::FullyEnforced => info!(
                        ?status,
                        "Landlock sandbox fully enforced for /app (filesystem access restricted)"
                    ),
                    RulesetStatus::PartiallyEnforced => info!(
                        ?status,
                        "Landlock sandbox partially enforced for /app (filesystem access restricted)"
                    ),
                    RulesetStatus::NotEnforced => {
                        warn!(
                            ?status,
                            "Landlock ruleset not enforced; kernel does not support requested features"
                        );
                    }
                }
            }
            Err(err) => {
                // Best-effort: log and continue without sandbox rather than crashing.
                warn!(?err, "Landlock not enabled; continuing without sandbox");
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub mod linux {
    /// No-op on non-Linux targets.
    pub fn init() {}
}

