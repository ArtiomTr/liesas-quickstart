use color_eyre::Result;
use color_eyre::{eyre::bail, owo_colors::colors::Default};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::ops::Range;
use std::{
    cell::OnceCell,
    collections::HashMap,
    path::{Path, PathBuf},
};
use toml::Spanned;

use crate::client::ClientKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ClientSource {
    Default(ClientKind),
    Binary { kind: ClientKind, bin: PathBuf },
    Image { kind: ClientKind, image: String },
}

impl ClientSource {
    fn kind(&self) -> ClientKind {
        match self {
            Self::Default(kind) => kind.clone(),
            Self::Binary { kind, .. } => kind.clone(),
            Self::Image { kind, .. } => kind.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeConfig {
    /// Name of docker container.
    ///
    /// If `count` is specified and is larger than 1, this name will be used as
    /// prefix, to generate container names for each node. For example,
    /// providing `name: "ream"` and `count: 2`, will start 2 nodes with names
    /// "ream_0" and "ream_1".
    #[serde(default)]
    name: Option<Spanned<String>>,

    /// Which client to use?
    ///
    /// This may be either shorthand alias
    client: Spanned<ClientSource>,

    /// How many exactly same nodes to launch.
    ///
    /// If you want to put few validators into node, then check
    /// `validator_count` parameter.
    #[serde(default = "default_count")]
    count: Spanned<u64>,

    /// How many validators should single node handle.
    ///
    /// Keep in mind that if you specify both `count` and `validator_count`, the
    /// total amount of validators, participating in network, would be
    /// `count * validator_count`. For example, if you specify: `name: "ream"`,
    /// `count: 3`, `validator_count: 5`, then network with 3 nodes ream_0,
    /// ream_1 and ream_2, with 5 validators on each node, so in total there
    /// would be 15 network participants.
    #[serde(default = "default_count")]
    validator_count: Spanned<u64>,

    /// Any extra command-line arguments to be passed directly into node binary.
    #[serde(default)]
    extra_args: Vec<String>,
}

/// default count, used for both `NodeConfig.count` and
/// `NodeConfig.validator_count`.
fn default_count() -> Spanned<u64> {
    Spanned::new(0..0, 1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Name of the network.
    name: String,

    node: Vec<NodeConfig>,
}

#[derive(Debug, Clone)]
struct ResolvedNodeConfig {}

#[derive(Debug, Clone)]
pub struct ResolvedNetworkConfig {
    nodes: HashMap<String, ResolvedNodeConfig>,
}

impl NetworkConfig {
    pub fn resolve(&self) -> Result<ResolvedNetworkConfig> {
        let mut resolved = HashMap::new();
        let mut by_prefix: HashMap<String, Vec<NodeConfig>> = HashMap::new();

        for node in self.node.iter() {
            if let Some(ref name) = node.name {
                if *node.count.as_ref() == 0 {
                    bail!("{:?} count cannot equal zero", node.count.span());
                } else if *node.count.as_ref() == 1 {
                    if resolved
                        .insert(name.clone(), ResolvedNodeConfig {})
                        .is_some()
                    {
                        bail!("node with name {name} appears twice in config");
                    };
                } else {
                    by_prefix
                        .entry(name.as_ref().clone())
                        .and_modify(|v| v.push(node.clone()))
                        .or_insert(vec![node.clone()]);
                }
            } else {
                by_prefix
                    .entry(node.client.as_ref().kind().to_string())
                    .and_modify(|v| v.push(node.clone()))
                    .or_insert(vec![node.clone()]);
            }
        }

        todo!()
    }
}
