mod cli;

use std::str::FromStr;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{
    args::Cli,
    commands::{Command, ConfigCommand, NetworkCommand},
    output::{init_tracing, print_version, run_repl},
};
use syncweb_core::{
    folder::{FolderManager, SyncMode},
    net::TransportFallback,
    node::{
        identity::{DeviceId, IdentityManager},
        iroh_node::{IrohNode, RelayMode},
    },
    storage::Config as AppConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    tracing::debug!(command = ?cli.command, "cli initialized");

    match cli.command {
        Command::Version => print_version(),
        Command::Repl => run_repl()?,
        Command::Create(command) => {
            std::fs::create_dir_all(&command.path).with_context(|| {
                format!("failed to create folder path {}", command.path.display())
            })?;
            let node = open_node(&cli.data_dir).await?;
            let manager = FolderManager::new(&node);
            let folder = manager.create(SyncMode::from_str(&command.mode)?).await?;
            let ticket = folder.ticket(node.endpoint().addr(), true).await?;
            println!("namespace: {}", folder.namespace_id());
            println!("ticket: {ticket}");
            node.stop().await?;
        }
        Command::Join(command) => {
            std::fs::create_dir_all(&command.path).with_context(|| {
                format!("failed to create folder path {}", command.path.display())
            })?;
            let node = open_node(&cli.data_dir).await?;
            let manager = FolderManager::new(&node);
            let folder = manager
                .join(command.ticket, SyncMode::from_str(&command.mode)?)
                .await?;
            println!("joined: {}", folder.namespace_id());
            node.stop().await?;
        }
        Command::Accept { namespace } => {
            let node = open_node(&cli.data_dir).await?;
            let manager = FolderManager::new(&node);
            let folder = manager.accept(namespace.parse()?).await?;
            println!("accepted: {}", folder.namespace_id());
            node.stop().await?;
        }
        Command::Drop { namespace } => {
            let node = open_node(&cli.data_dir).await?;
            let manager = FolderManager::new(&node);
            manager.drop(namespace.parse()?).await?;
            println!("dropped: {namespace}");
            node.stop().await?;
        }
        Command::Folders => {
            let node = open_node(&cli.data_dir).await?;
            let manager = FolderManager::new(&node);
            for folder in manager.list().await? {
                println!("{}\t{}", folder.namespace_id(), folder.mode());
            }
            node.stop().await?;
        }
        Command::Devices => {
            let identity = IdentityManager::new(cli.data_dir.join("identity.key"))?;
            let device_id = DeviceId::from_node_id(identity.node_id());
            println!("iroh: {}", identity.node_id());
            println!("syncthing: {}", device_id.to_syncthing());
        }
        Command::Config { command } => {
            let config_path = cli.data_dir.join("config.toml");
            let mut config = AppConfig::load(&config_path)?;
            match command {
                None | Some(ConfigCommand::Show { section: None }) => {
                    print_config(&config)?;
                }
                Some(ConfigCommand::Show {
                    section: Some(section),
                }) => {
                    if section != "bep" {
                        anyhow::bail!(
                            "unsupported config section {section:?}; supported section: bep"
                        );
                    }
                    println!("{}", toml::to_string_pretty(&config.bep)?);
                }
                Some(ConfigCommand::Set { key, value }) => {
                    config.set(&key, &value)?;
                    config.save(&config_path)?;
                    println!("{key} updated");
                }
            }
        }
        Command::Network {
            command: NetworkCommand::TestRelay { relay_url },
        } => {
            let identity = IdentityManager::new(cli.data_dir.join("identity.key"))?;
            let app_config = AppConfig::load(cli.data_dir.join("config.toml"))?;
            let mut config = app_config.relay_config();
            config.relay_urls = vec![relay_url.clone()];
            config.auto_fallback = true;
            TransportFallback::new(config)
                .connect_relay(DeviceId::from_node_id(identity.node_id()))
                .await?;
            println!("relay reachable: {relay_url}");
        }
    }
    Ok(())
}

fn print_config(config: &AppConfig) -> Result<()> {
    print!("{}", toml::to_string_pretty(config)?);
    Ok(())
}

async fn open_node(data_dir: &std::path::Path) -> Result<IrohNode> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    IrohNode::new(identity, data_dir.join("data"), RelayMode::Default).await
}
