use ariadne::{Label, ReportKind};
use leansig::serialization::Serializable;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ops::Range;
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;
use toml::Spanned;

use crate::client::ClientKind;
use crate::validator::{PrivateKey, generate_keypair};

pub type Span = Range<usize>;

#[derive(Clone, Debug)]
pub enum NodeNameSource {
    Name(Span),
    Kind(Span),
}

impl NodeNameSource {
    pub fn span(&self) -> Span {
        match self {
            Self::Name(span) => span.clone(),
            Self::Kind(span) => span.clone(),
        }
    }
}

#[derive(Debug)]
pub enum NodeNameDefinition {
    Singular(NodeNameSource),
    Prefix {
        prefix: String,
        prefix_span: NodeNameSource,
        count_span: Span,
    },
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("count cannot equal to zero")]
    InvalidCount(Span),

    #[error("the name `{name}` is defined multiple times")]
    DuplicateName {
        name: String,
        span: Span,
        previous: NodeNameDefinition,
    },
}

impl ConfigError {
    pub fn span(&self) -> Span {
        match self {
            Self::InvalidCount(span) => span.clone(),
            Self::DuplicateName { span, .. } => span.clone(),
        }
    }

    pub fn report(&self, file: PathBuf) -> ariadne::Report<'_, (String, Span)> {
        let file = file.display().to_string();
        let mut builder = ariadne::Report::build(ReportKind::Error, (file.clone(), self.span()));

        match self {
            Self::InvalidCount(span) => {
                builder = builder
                    .with_message("Invalid node configuration")
                    .with_label(
                        Label::new((file.clone(), span.clone()))
                            .with_message("cannot equal to zero"),
                    );
            }
            Self::DuplicateName {
                name,
                span,
                previous,
            } => {
                builder =
                    builder.with_message(format!("the name `{name}` is defined multiple times"));

                match previous {
                    NodeNameDefinition::Singular(kind) => {
                        let (span, message) = match kind {
                            NodeNameSource::Name(span) => {
                                (span.clone(), format!("previous definition here"))
                            }
                            NodeNameSource::Kind(span) => (
                                span.clone(),
                                format!("previous definition derived from client kind here"),
                            ),
                        };

                        builder = builder
                            .with_label(Label::new((file.clone(), span)).with_message(message));
                    }
                    NodeNameDefinition::Prefix {
                        prefix,
                        prefix_span,
                        count_span,
                    } => {
                        let (prefix_span, message) = match prefix_span {
                            NodeNameSource::Name(span) => {
                                (span.clone(), format!("prefix `{prefix}` defined here"))
                            }
                            NodeNameSource::Kind(span) => (
                                span.clone(),
                                format!("prefix `{prefix}` derived from client kind here"),
                            ),
                        };

                        builder = builder
                            .with_label(
                                Label::new((file.clone(), prefix_span)).with_message(message),
                            )
                            .with_label(
                                Label::new((file.clone(), count_span.clone()))
                                    .with_message("count defined here"),
                            )
                            .with_note(format!("the generated name `{name}` comes from prefix `{prefix}_` and index {}", name.strip_prefix(&format!("{prefix}_")).unwrap_or("<unknown>")));
                    }
                }

                builder = builder.with_label(
                    Label::new((file.clone(), span.clone())).with_message("redefined here"),
                );
            }
        }

        builder.finish()
    }
}

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
    #[serde(default = "default_validator_count")]
    validator_count: u64,

    /// Any extra command-line arguments to be passed directly into node binary.
    #[serde(default)]
    extra_args: Vec<String>,
}

/// default value, used for `NodeConfig.count`.
fn default_count() -> Spanned<u64> {
    Spanned::new(0..0, 1)
}

/// default value, user for `NodeConfig.validator_count`
fn default_validator_count() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Name of the network.
    name: String,

    node: Vec<NodeConfig>,
}

#[derive(Debug, Clone)]
struct ResolvedNodeConfig {
    validators: Vec<usize>,
}

#[derive(Debug, Clone)]
struct ResolvedValidatorConfig {
    private_key: Vec<u8>,
    public_key: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ResolvedNetworkConfig {
    validators: Vec<ResolvedValidatorConfig>,
    nodes: HashMap<String, ResolvedNodeConfig>,
}

const NUM_ACTIVE_EPOCHS: usize = 262144;

impl ResolvedNetworkConfig {
    fn resolve(&mut self, node: NodeConfig) -> Result<(), ConfigError> {
        let mut validator_indices = Vec::new();
        for _ in 0..node.validator_count {
            validator_indices.push(self.validators.len());

            let (private_key, public_key) = generate_keypair(0, NUM_ACTIVE_EPOCHS);

            self.validators.push(ResolvedValidatorConfig {
                private_key: private_key.to_bytes(),
                public_key: public_key.to_bytes(),
            });
        }

        let (node_id, node_id_span) = node
            .name
            .as_ref()
            .map(|v| (v.get_ref().clone(), NodeNameSource::Name(v.span())))
            .unwrap_or_else(|| {
                (
                    node.client.get_ref().kind().to_string(),
                    NodeNameSource::Kind(node.client.span()),
                )
            });

        let resolved = ResolvedNodeConfig {
            validators: validator_indices,
        };

        if let Some(old) = self.nodes.insert(node_id, resolved) {}

        Ok(())
    }
}

impl NetworkConfig {
    pub fn resolve(&self) -> Result<ResolvedNetworkConfig, ConfigError> {
        let mut resolved = HashMap::new();
        let mut by_prefix: HashMap<String, Vec<(NodeNameSource, NodeConfig)>> = HashMap::new();

        for node in self.node.iter() {
            let (name, span) = node
                .name
                .as_ref()
                .map(|v| (v.get_ref().clone(), NodeNameSource::Name(v.span())))
                .unwrap_or_else(|| {
                    (
                        node.client.get_ref().kind().to_string(),
                        NodeNameSource::Kind(node.client.span()),
                    )
                });

            if *node.count.as_ref() == 0 {
                return Err(ConfigError::InvalidCount(node.count.span()));
            } else if *node.count.as_ref() == 1 {
                if let Some((previous, _)) =
                    resolved.insert(name.clone(), (span.clone(), ResolvedNodeConfig {}))
                {
                    return Err(ConfigError::DuplicateName {
                        name: name,
                        span: span.span(),
                        previous: NodeNameDefinition::Singular(previous),
                    });
                };
            } else {
                by_prefix
                    .entry(name)
                    .and_modify(|v| v.push((span.clone(), node.clone())))
                    .or_insert(vec![(span, node.clone())]);
            }
        }

        let mut validators = Vec::new();

        for (group_prefix, configs) in by_prefix {
            let mut counter = 0;

            for (prefix_span, config) in configs {
                let count_span = config.count.span();
                let count = config.count.into_inner();

                debug_assert!(count > 1, "count must be greater than 1 at this point");

                for _ in 0..count {
                    let name = format!("{group_prefix}_{counter}");

                    if let Some((duplicate, _)) = resolved.insert(
                        name.clone(),
                        (NodeNameSource::Kind(0..0), ResolvedNodeConfig {}),
                    ) {
                        return Err(ConfigError::DuplicateName {
                            name,
                            span: duplicate.span(),
                            previous: NodeNameDefinition::Prefix {
                                prefix: group_prefix,
                                prefix_span,
                                count_span,
                            },
                        });
                    }
                    counter += 1;
                }
            }
        }

        Ok(ResolvedNetworkConfig {
            nodes: resolved
                .into_iter()
                .map(|(key, value)| (key, value.1))
                .collect(),
        })
    }
}
