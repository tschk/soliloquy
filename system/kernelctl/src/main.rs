use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

const DEFAULT_POLICY: &str = "/etc/soliloquy/kernel-policy.json";
const DEFAULT_RUNTIME_STATE: &str = "/run/soliloquy/runtime-state.env";
const DEFAULT_CGROUP_FS: &str = "/sys/fs/cgroup";

#[derive(Debug, Deserialize)]
struct KernelPolicy {
    profile: String,
    groups: Vec<CgroupPolicy>,
    sysctl: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct CgroupPolicy {
    id: String,
    path: String,
    cpu_weight: Option<u64>,
    io_weight: Option<u64>,
    memory_high: Option<String>,
    memory_max: Option<String>,
    pids_max: Option<u64>,
}

#[derive(Debug)]
struct Args {
    policy: PathBuf,
    runtime_state: PathBuf,
    cgroup_fs: PathBuf,
    dry_run: bool,
}

fn main() {
    if let Err(error) = run(parse_args(env::args().skip(1))) {
        eprintln!("sol-kernelctl: {error}");
        std::process::exit(1);
    }
}

fn parse_args<I>(args: I) -> Args
where
    I: IntoIterator<Item = String>,
{
    let mut policy = env::var_os("SOLILOQUY_KERNEL_POLICY_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_POLICY));
    let mut runtime_state = env::var_os("SOLILOQUY_RUNTIME_STATE_ENV")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RUNTIME_STATE));
    let mut cgroup_fs = env::var_os("SOLILOQUY_CGROUP_FS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CGROUP_FS));
    let mut dry_run = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--policy" => {
                if let Some(value) = iter.next() {
                    policy = PathBuf::from(value);
                }
            }
            "--runtime-state" => {
                if let Some(value) = iter.next() {
                    runtime_state = PathBuf::from(value);
                }
            }
            "--cgroup-fs" => {
                if let Some(value) = iter.next() {
                    cgroup_fs = PathBuf::from(value);
                }
            }
            "--dry-run" => dry_run = true,
            _ => {}
        }
    }
    Args {
        policy,
        runtime_state,
        cgroup_fs,
        dry_run,
    }
}

fn run(args: Args) -> Result<(), String> {
    let policy = read_policy(&args.policy)?;
    for module in ["virtio_pci", "virtio_net", "virtio_rng", "virtio_gpu"] {
        load_kernel_module(module, args.dry_run);
    }
    for (key, value) in &policy.sysctl {
        apply_sysctl(key, value, args.dry_run).map_err(|error| format!("{key}: {error}"))?;
    }
    let cgroups_state = apply_cgroups(&policy.groups, &args.cgroup_fs, args.dry_run)
        .map_err(|error| format!("cgroup policy: {error}"))?;
    record_runtime_state(
        &args.runtime_state,
        "SOLILOQUY_KERNEL_POLICY_FILE",
        args.policy.display(),
    )
    .map_err(|error| format!("runtime state: {error}"))?;
    record_runtime_state(
        &args.runtime_state,
        "SOLILOQUY_KERNEL_POLICY_CGROUPS",
        cgroups_state,
    )
    .map_err(|error| format!("runtime state: {error}"))?;
    record_runtime_state(
        &args.runtime_state,
        "SOLILOQUY_KERNEL_POLICY_PROFILE",
        &policy.profile,
    )
    .map_err(|error| format!("runtime state: {error}"))?;
    Ok(())
}

fn read_policy(path: &Path) -> Result<KernelPolicy, String> {
    let raw =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_str(&raw).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn load_kernel_module(module: &str, dry_run: bool) {
    if dry_run {
        return;
    }
    let _ = Command::new("modprobe").arg(module).status();
}

fn apply_sysctl(key: &str, value: &str, dry_run: bool) -> io::Result<()> {
    if dry_run {
        return Ok(());
    }
    let path = Path::new("/proc/sys").join(key.replace('.', "/"));
    if path.exists() {
        fs::write(path, format!("{value}\n"))?;
    }
    Ok(())
}

fn apply_cgroups<'a>(
    groups: &'a [CgroupPolicy],
    cgroup_fs: &Path,
    dry_run: bool,
) -> io::Result<&'a str> {
    if !cgroup_fs.join("cgroup.controllers").exists() {
        return Ok("unavailable");
    }
    if dry_run {
        return Ok("active");
    }
    fs::create_dir_all(cgroup_fs.join("soliloquy"))?;
    enable_controllers(cgroup_fs);
    for group in groups {
        apply_group(cgroup_fs, group)?;
    }
    Ok("active")
}

fn enable_controllers(cgroup_fs: &Path) {
    let subtree_control = cgroup_fs.join("cgroup.subtree_control");
    for controller in ["cpu", "io", "memory", "pids"] {
        let _ = fs::write(&subtree_control, format!("+{controller}\n"));
    }
}

fn apply_group(cgroup_fs: &Path, group: &CgroupPolicy) -> io::Result<()> {
    let path = cgroup_fs.join(&group.path);
    fs::create_dir_all(&path)?;
    write_optional(&path, "cpu.weight", group.cpu_weight)?;
    write_optional(&path, "io.weight", group.io_weight)?;
    write_optional_string(&path, "memory.high", group.memory_high.as_deref())?;
    write_optional_string(&path, "memory.max", group.memory_max.as_deref())?;
    write_optional(&path, "pids.max", group.pids_max)?;
    record_group_id(&path, &group.id)?;
    Ok(())
}

fn write_optional(path: &Path, file: &str, value: Option<u64>) -> io::Result<()> {
    if let Some(value) = value {
        write_optional_string(path, file, Some(&value.to_string()))?;
    }
    Ok(())
}

fn write_optional_string(path: &Path, file: &str, value: Option<&str>) -> io::Result<()> {
    let target = path.join(file);
    if let Some(value) = value {
        if target.exists() {
            fs::write(target, format!("{value}\n"))?;
        }
    }
    Ok(())
}

fn record_group_id(path: &Path, id: &str) -> io::Result<()> {
    let target = path.join("soliloquy.group");
    fs::write(target, format!("{id}\n"))
}

fn record_runtime_state(path: &Path, key: &str, value: impl std::fmt::Display) -> io::Result<()> {
    let mut values = BTreeMap::new();
    if let Ok(raw) = fs::read_to_string(path) {
        for line in raw.lines() {
            if let Some((existing_key, existing_value)) = line.split_once('=') {
                values.insert(existing_key.to_string(), existing_value.to_string());
            }
        }
    }
    values.insert(key.to_string(), value.to_string());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut rendered = String::new();
    for (key, value) in values {
        rendered.push_str(&key);
        rendered.push('=');
        rendered.push_str(&value);
        rendered.push('\n');
    }
    fs::write(path, rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_cli_overrides() {
        let args = parse_args([
            "--policy".to_string(),
            "/tmp/policy.json".to_string(),
            "--runtime-state".to_string(),
            "/tmp/state.env".to_string(),
            "--cgroup-fs".to_string(),
            "/tmp/cgroup".to_string(),
            "--dry-run".to_string(),
        ]);
        assert_eq!(args.policy, PathBuf::from("/tmp/policy.json"));
        assert_eq!(args.runtime_state, PathBuf::from("/tmp/state.env"));
        assert_eq!(args.cgroup_fs, PathBuf::from("/tmp/cgroup"));
        assert!(args.dry_run);
    }

    #[test]
    fn dry_run_records_policy_state() {
        let root = temp_root("sol-kernelctl-dry-run");
        fs::create_dir_all(root.join("cgroup")).unwrap();
        fs::write(
            root.join("cgroup/cgroup.controllers"),
            "cpu io memory pids\n",
        )
        .unwrap();
        let policy = root.join("policy.json");
        fs::write(
            &policy,
            r#"{
              "profile": "internet-appliance",
              "groups": [{"id":"renderer","path":"soliloquy/renderer","cpu_weight":800}],
              "sysctl": {"net.core.somaxconn": "4096"}
            }"#,
        )
        .unwrap();
        let runtime_state = root.join("state.env");
        run(Args {
            policy,
            runtime_state: runtime_state.clone(),
            cgroup_fs: root.join("cgroup"),
            dry_run: true,
        })
        .unwrap();
        let state = fs::read_to_string(runtime_state).unwrap();
        assert!(state.contains("SOLILOQUY_KERNEL_POLICY_PROFILE=internet-appliance"));
        assert!(state.contains("SOLILOQUY_KERNEL_POLICY_CGROUPS=active"));
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("{name}-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
