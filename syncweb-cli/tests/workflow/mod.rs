mod basic_sync;
mod indexing;
mod transfer;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Context, ensure};

fn syncweb_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
}

pub struct CmdOutput(pub Output);

impl CmdOutput {
    pub fn success(&self) -> bool {
        self.0.status.success()
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.0.stdout).into_owned()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.0.stderr).into_owned()
    }

    pub fn assert_success(&self, label: &str) -> anyhow::Result<()> {
        ensure!(
            self.success(),
            "{label} failed (exit {}):\nstdout: {}\nstderr: {}",
            self.0.status,
            self.stdout(),
            self.stderr()
        );
        Ok(())
    }
}

pub struct TicketInfo {
    pub ticket: String,
    pub namespace: String,
}

impl TicketInfo {
    pub fn from_stdout(stdout: &str) -> anyhow::Result<Self> {
        let mut ticket = None;
        let mut namespace = None;
        for line in stdout.lines() {
            if let Some(value) = line.strip_prefix("ticket: ") {
                ticket = Some(value.trim().to_string());
            }
            if let Some(value) = line.strip_prefix("namespace: ") {
                namespace = Some(value.trim().to_string());
            }
        }
        Ok(Self {
            ticket: ticket.context("no ticket found in output")?,
            namespace: namespace.context("no namespace found in output")?,
        })
    }
}

pub struct Device {
    name: String,
    data_dir: PathBuf,
}

impl Device {
    pub fn new(name: &str, root: &Path) -> anyhow::Result<Self> {
        let data_dir = root.join(format!("data-{name}"));
        std::fs::create_dir_all(&data_dir).with_context(|| format!("create data dir for {name}"))?;
        Ok(Self {
            name: name.to_string(),
            data_dir,
        })
    }

    pub fn run(&self, args: &[&str]) -> anyhow::Result<CmdOutput> {
        let mut all_args = vec!["--data-dir", self.data_dir.to_str().context("UTF-8 path")?];
        all_args.extend_from_slice(args);
        let output = syncweb_bin()
            .args(&all_args)
            .output()
            .with_context(|| format!("run syncweb {:?} as {}", args, self.name))?;
        Ok(CmdOutput(output))
    }

    pub fn run_ok(&self, args: &[&str]) -> anyhow::Result<CmdOutput> {
        let output = self.run(args)?;
        output.assert_success(&format!("{}: syncweb {:?}", self.name, args))?;
        Ok(output)
    }

    pub fn create(&self, path: &Path) -> anyhow::Result<TicketInfo> {
        let output = self.run_ok(&["--no-daemon", "create", path.to_str().context("UTF-8 path")?])?;
        let namespace = output
            .stdout()
            .lines()
            .find_map(|line| line.strip_prefix("namespace: "))
            .map(str::trim)
            .context("create should print a namespace")?
            .to_owned();
        let share = self.run_ok(&["--no-daemon", "share", "--namespace", &namespace, "--write"])?;
        TicketInfo::from_stdout(&share.stdout())
    }

    pub fn join(&self, ticket: &str, path: &Path) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["--no-daemon", "join", ticket, path.to_str().context("UTF-8 path")?])
    }

    pub fn join_with_options(&self, args: &[&str], ticket: &str, path: &Path) -> anyhow::Result<CmdOutput> {
        let mut all = vec!["--no-daemon", "join"];
        all.extend_from_slice(args);
        all.push(ticket);
        all.push(path.to_str().context("UTF-8 path")?);
        self.run_ok(&all)
    }

    pub fn leave(&self, namespace: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["--no-daemon", "leave", namespace])
    }

    #[expect(dead_code, reason = "part of DSL public API")]
    pub fn leave_delete_files(&self, namespace: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["--no-daemon", "leave", namespace, "--delete-files"])
    }

    #[expect(clippy::unused_self, reason = "API consistency")]
    pub fn write_file(&self, path: &Path, content: &[u8]) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("create parent dirs for {}", path.display()))?;
        }
        std::fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    pub fn import(&self, path: &Path) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["--no-daemon", "import", path.to_str().context("UTF-8 path")?])
    }

    pub fn ls(&self, path: &Path) -> anyhow::Result<Vec<String>> {
        let output = self.run_ok(&["ls", path.to_str().context("UTF-8 path")?])?;
        Ok(output
            .stdout()
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    pub fn find(&self, pattern: &str, path: &Path) -> anyhow::Result<Vec<String>> {
        let output = self.run_ok(&["find", pattern, path.to_str().context("UTF-8 path")?])?;
        Ok(output
            .stdout()
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    pub fn folders(&self) -> anyhow::Result<Vec<String>> {
        let output = self.run_ok(&["--no-daemon", "folders"])?;
        Ok(output
            .stdout()
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    #[expect(clippy::unused_self, reason = "API consistency")]
    pub fn file_content(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        std::fs::read(path).with_context(|| format!("read {}", path.display()))
    }

    pub fn stat(&self, path: &Path) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["stat", path.to_str().context("UTF-8 path")?])
    }

    pub fn verify(&self, path: &Path) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["--no-daemon", "verify", path.to_str().context("UTF-8 path")?])
    }

    pub fn config_show(&self) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["config", "show"])
    }

    pub fn config_set(&self, key: &str, value: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["config", "set", key, value])
    }

    pub fn network_create(&self, name: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["network", "create", name])
    }

    pub fn network_list(&self) -> anyhow::Result<Vec<String>> {
        let output = self.run_ok(&["network", "ls"])?;
        Ok(output
            .stdout()
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    pub fn network_invite(&self, name: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["network", "invite", name])
    }

    pub fn network_leave(&self, name: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["network", "leave", name])
    }

    pub fn snapshot_create(&self, path: &Path) -> anyhow::Result<CmdOutput> {
        self.run_ok(&[
            "--no-daemon",
            "snapshot",
            "create",
            path.to_str().context("UTF-8 path")?,
        ])
    }

    pub fn snapshot_create_described(
        &self,
        path: &Path,
        description: &str,
        threads: &str,
    ) -> anyhow::Result<CmdOutput> {
        self.run_ok(&[
            "--no-daemon",
            "snapshot",
            "create",
            path.to_str().context("UTF-8 path")?,
            "--description",
            description,
            "--threads",
            threads,
        ])
    }

    pub fn snapshot_create_id(&self, path: &Path) -> anyhow::Result<String> {
        let output = self.snapshot_create(path)?;
        output
            .stdout()
            .lines()
            .find_map(|line| line.strip_prefix("snapshot: "))
            .map(str::trim)
            .map(String::from)
            .context("snapshot id missing from create output")
    }

    pub fn snapshot_restore(&self, destination: &Path, snapshot_id: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&[
            "--no-daemon",
            "snapshot",
            "restore",
            destination.to_str().context("UTF-8 path")?,
            snapshot_id,
        ])
    }

    pub fn snapshot_diff(&self, path: &Path, first: &str, second: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&[
            "--no-daemon",
            "snapshot",
            "diff",
            path.to_str().context("UTF-8 path")?,
            first,
            second,
        ])
    }

    pub fn snapshot_delete(&self, path: &Path, snapshot_id: &str) -> anyhow::Result<CmdOutput> {
        self.run_ok(&[
            "--no-daemon",
            "snapshot",
            "delete",
            path.to_str().context("UTF-8 path")?,
            snapshot_id,
        ])
    }

    pub fn snapshot_list(&self) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["--no-daemon", "snapshot", "list"])
    }

    pub fn snapshot_list_json(&self) -> anyhow::Result<serde_json::Value> {
        let output = self.run_ok(&["--json", "--no-daemon", "snapshot", "list"])?;
        serde_json::from_str(&output.stdout()).context("parse snapshot list JSON")
    }

    pub fn stats_network(&self) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["stats", "network"])
    }

    pub fn db_check(&self) -> anyhow::Result<CmdOutput> {
        self.run_ok(&["db", "check"])
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct World {
    root: PathBuf,
    devices: Vec<Device>,
}

impl World {
    pub fn new(device_names: &[&str]) -> anyhow::Result<Self> {
        let root = std::env::temp_dir().join(format!("syncweb-workflow-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root)?;

        let mut devices = Vec::new();
        for name in device_names {
            devices.push(Device::new(name, &root)?);
        }

        Ok(Self { root, devices })
    }

    pub fn device(&self, name: &str) -> anyhow::Result<&Device> {
        self.devices.iter().find(|d| d.name() == name).with_context(|| {
            let available: Vec<_> = self.devices.iter().map(Device::name).collect();
            format!("device '{name}' not found; available: {available:?}")
        })
    }

    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
