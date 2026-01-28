use ariadne::{Label, ReportKind};
use leansig::serialization::Serializable;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ops::Range;
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;
use toml::Spanned;

use crate::client::ClientKind;
use crate::validator::generate_keypair;

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

#[derive(Debug, Clone)]
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
        curr_def: NodeNameDefinition,
        prev_def: NodeNameDefinition,
    },
}

impl ConfigError {
    pub fn span(&self) -> Span {
        match self {
            Self::InvalidCount(span) => span.clone(),
            Self::DuplicateName { curr_def, .. } => match curr_def {
                NodeNameDefinition::Singular(source) => source.span(),
                NodeNameDefinition::Prefix { prefix_span, .. } => prefix_span.span(),
            },
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
                curr_def,
                prev_def,
            } => {
                builder =
                    builder.with_message(format!("the name `{name}` is defined multiple times"));

                match prev_def {
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
                            .with_note(format!("the first generated name `{name}` comes from prefix `{prefix}_` and index {}", name.strip_prefix(&format!("{prefix}_")).unwrap_or("<unknown>")));
                    }
                }

                match curr_def {
                    NodeNameDefinition::Singular(kind) => {
                        let (span, message) = match kind {
                            NodeNameSource::Name(span) => {
                                (span.clone(), format!("second definition appears here"))
                            }
                            NodeNameSource::Kind(span) => (
                                span.clone(),
                                format!("second definition appears here, derived from client"),
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
                            .with_note(format!("the redefinition name `{name}` comes from prefix `{prefix}_` and index {}", name.strip_prefix(&format!("{prefix}_")).unwrap_or("<unknown>")));
                        }
                }
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
    def: NodeNameDefinition,

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
    counters: HashMap<String, u64>,
}

const NUM_ACTIVE_EPOCHS: usize = 262144;

impl ResolvedNetworkConfig {
    fn resolve(&mut self, node: NodeConfig) -> Result<(), ConfigError> {
        let count = *node.count.get_ref();

        if count == 0 {
            return Err(ConfigError::InvalidCount(node.count.span()));
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

        for _ in 0..count {
            let mut validator_indices = Vec::new();
            for _ in 0..node.validator_count {
                validator_indices.push(self.validators.len());

                // let (private_key, public_key) = generate_keypair(0, NUM_ACTIVE_EPOCHS);
                let (private_key, public_key) = (Vec::new(), Vec::new());

                self.validators.push(ResolvedValidatorConfig {
                    private_key: private_key,
                    public_key: public_key,
                });
            }

            let (name, def) = if count == 1 {
                (
                    node_id.clone(),
                    NodeNameDefinition::Singular(node_id_span.clone()),
                )
            } else {
                let index = *self
                    .counters
                    .entry(node_id.clone())
                    .and_modify(|v| *v += 1)
                    .or_insert(0);
                (
                    format!("{}_{index}", node_id.clone()),
                    NodeNameDefinition::Prefix {
                        prefix: node_id.clone(),
                        prefix_span: node_id_span.clone(),
                        count_span: node.count.span(),
                    },
                )
            };

            let resolved = ResolvedNodeConfig {
                def: def.clone(),
                validators: validator_indices,
            };

            if let Some(old) = self.nodes.insert(name.clone(), resolved) {
                return Err(ConfigError::DuplicateName {
                    name,
                    curr_def: def,
                    prev_def: old.def,
                });
            }
        }

        Ok(())
    }
}

impl NetworkConfig {
    pub fn resolve(self) -> Result<ResolvedNetworkConfig, ConfigError> {
        let mut resolved = ResolvedNetworkConfig {
            nodes: HashMap::new(),
            validators: Vec::new(),
            counters: HashMap::new(),
        };

        for node in self.node.into_iter() {
            resolved.resolve(node)?;
        }

        Ok(resolved)
    }
}
