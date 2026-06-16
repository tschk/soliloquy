//! Desktop status aggregator.
//! Reads alpenglow daemon state files and composes a unified status.

use std::path::Path;

use serde::Deserialize;

/// Paths to alpenglow daemon runtime state files.
#[derive(Clone)]
pub struct AlpenglowPaths {
    pub netd_json: String,
    pub runtime_env: String,
}

impl Default for AlpenglowPaths {
    fn default() -> Self {
        Self {
            netd_json: "/run/alpenglow/netd/interfaces.json".into(),
            runtime_env: "/run/alpenglow/runtime-state.env".into(),
        }
    }
}

/// Aggregated desktop environment status.
#[derive(Debug, Default, serde::Serialize)]
pub struct DesktopStatus {
    pub network: NetworkStatus,
    pub kernel: KernelStatus,
    pub session: SessionInfo,
    pub apps: Vec<AppInfo>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct NetworkStatus {
    pub default_interface: String,
    pub interfaces_up: usize,
    pub interfaces_total: usize,
    pub has_connectivity: bool,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct KernelStatus {
    pub profile: String,
    pub cgroup_state: String,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct SessionInfo {
    pub user: String,
    pub display: String,
    pub uptime_secs: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub pid: Option<u32>,
    pub state: String,
}

/// Local deserialization mirror of alpenglow netd JSON format.
/// (alpenglow-netd types don't derive Deserialize — we parse here instead.)
#[derive(Deserialize)]
struct NetdSnapshot {
    interfaces: Vec<NetdInterface>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct NetdInterface {
    name: String,
    operstate: String,
}

impl DesktopStatus {
    /// Read alpenglow netd state from its JSON snapshot file.
    fn read_network_state(paths: &AlpenglowPaths) -> NetworkStatus {
        let json_path = Path::new(&paths.netd_json);
        let content = match std::fs::read_to_string(json_path) {
            Ok(c) => c,
            Err(_) => return NetworkStatus::default(),
        };
        let snapshot: NetdSnapshot = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(_) => return NetworkStatus::default(),
        };

        let total = snapshot.interfaces.len();
        let up = snapshot
            .interfaces
            .iter()
            .filter(|iface| iface.operstate == "up")
            .count();
        let default_iface = snapshot
            .interfaces
            .iter()
            .find(|iface| iface.name != "lo" && iface.operstate == "up")
            .map(|iface| iface.name.clone())
            .unwrap_or_default();

        NetworkStatus {
            interfaces_total: total,
            interfaces_up: up,
            default_interface: default_iface,
            has_connectivity: up > 0,
        }
    }

    /// Read alpenglow kernelctl state from its env file.
    fn read_kernel_state(paths: &AlpenglowPaths) -> KernelStatus {
        let env_path = Path::new(&paths.runtime_env);
        let content = match std::fs::read_to_string(env_path) {
            Ok(c) => c,
            Err(_) => return KernelStatus::default(),
        };

        let mut profile = String::new();
        let mut cgroup_state = String::new();

        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "ALPENGLOW_KERNEL_POLICY_PROFILE" => profile = value.to_string(),
                    "ALPENGLOW_KERNEL_POLICY_CGROUPS" => cgroup_state = value.to_string(),
                    _ => {}
                }
            }
        }

        KernelStatus {
            profile,
            cgroup_state,
        }
    }

    /// Collect full desktop status.
    pub fn collect(paths: &AlpenglowPaths) -> Self {
        DesktopStatus {
            network: Self::read_network_state(paths),
            kernel: Self::read_kernel_state(paths),
            session: SessionInfo::default(),
            apps: Vec::new(),
        }
    }
}
