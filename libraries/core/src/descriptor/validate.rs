use crate::{
    adjust_shared_library_path,
    config::{DataId, Input, InputMapping, OperatorId, UserInputMapping},
    descriptor::{self, source_is_url, CoreNodeKind, OperatorSource, EXE_EXTENSION},
    get_python_path,
};

use eyre::{bail, eyre, Context};
use std::collections::HashSet;
use std::{path::Path, process::Command};
use tracing::info;

use super::{resolve_path, source_to_path,  DYNAMIC_SOURCE, Descriptor, SHELL_SOURCE};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn check_dataflow(
    dataflow: &Descriptor,
    working_dir: &Path,
    remote_daemon_id: Option<&[&str]>,
    coordinator_is_remote: bool,
) -> eyre::Result<()> {
    let nodes = dataflow.resolve_aliases_and_set_defaults()?;
    let mut has_python_operator = false;
    let is_local = nodes
        .iter()
        .map(|n| &n.deploy.machine)
        .collect::<HashSet<_>>()
        .len()
        <= 1;

    // check that nodes and operators exist
    for node in &nodes {
        match &node.kind {
            descriptor::CoreNodeKind::Custom(custom) => match custom.source.as_str() {
                SHELL_SOURCE => (),
                DYNAMIC_SOURCE => (),
                source => {
                    if source_is_url(source) {
                        info!("{source} is a URL."); // TODO: Implement url check.
                      
                    } else if is_local {

                        resolve_path(source, working_dir)
                            .wrap_err_with(|| format!("Could not find source path `{}`", source))?;
                    } else {
                        let path = source_to_path(source);
                        if path.is_relative() {
                            eyre::bail!(
                                "paths of remote nodes must be absolute (node `{}`)",
                                node.id
                            );
                        }
                        info!("skipping path check for remote node `{}`", node.id);
                    }
                }
            },
            descriptor::CoreNodeKind::Runtime(node) => {
                for operator_definition in &node.operators {
                    match &operator_definition.config.source {
                        OperatorSource::SharedLibrary(path) => {
                            if source_is_url(path) {
                                info!("{path} is a URL."); // TODO: Implement url check.
                            } else if is_local {
                                let path = adjust_shared_library_path(Path::new(&path))?;
                                if !working_dir.join(&path).exists() {
                                    bail!("no shared library at `{}`", path.display());
                                }
                            } else {
                                let path = source_to_path(path);
                                if path.is_relative() {
                                    eyre::bail!(
                                        "paths of operator must be absolute (operator `{}`)",
                                        operator_definition.id
                                    );
                                }
                                info!(
                                    "skipping path check for remote operator `{}`",
                                    operator_definition.id
                                );
                            }
                        }
                        OperatorSource::Python(python_source) => {
                            has_python_operator = true;
                            let path = &python_source.source;
                            if source_is_url(path) {
                                info!("{path} is a URL."); // TODO: Implement url check.
                            } else if is_local {
                                if !working_dir.join(path).exists() {
                                    bail!("no Python library at `{path}`");
                                }
                            } else {
                                let path = source_to_path(path);
                                if path.is_relative() {
                                    eyre::bail!(
                                        "paths of python operator must be absolute (operator `{}`)",
                                        operator_definition.id
                                    );
                                }
                                info!(
                                    "skipping path check for remote python operator `{}`",
                                    operator_definition.id
                                );
                            }
                        }
                        OperatorSource::Wasm(path) => {
                            if source_is_url(path) {
                                info!("{path} is a URL."); // TODO: Implement url check.
                            } else if is_local {
                                if !working_dir.join(path).exists() {
                                    bail!("no WASM library at `{path}`");
                                }
                            } else {
                                let path = source_to_path(path);
                                if path.is_relative() {
                                    eyre::bail!(
                                        "paths of Wasm operator must be absolute (operator `{}`)",
                                        operator_definition.id
                                    );
                                }
                                info!(
                                    "skipping path check for remote Wasm operator `{}`",
                                    operator_definition.id
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // check that all inputs mappings point to an existing output
    for node in &nodes {
        match &node.kind {
            descriptor::CoreNodeKind::Custom(custom_node) => {
                for (input_id, input) in &custom_node.run_config.inputs {
                    check_input(input, &nodes, &format!("{}/{input_id}", node.id))?;
                }
            }
            descriptor::CoreNodeKind::Runtime(runtime_node) => {
                for operator_definition in &runtime_node.operators {
                    for (input_id, input) in &operator_definition.config.inputs {
                        check_input(
                            input,
                            &nodes,
                            &format!("{}/{}/{input_id}", operator_definition.id, node.id),
                        )?;
                    }
                }
            }
        };
    }

    // Check that nodes can resolve `send_stdout_as`
    for node in &nodes {
        node.send_stdout_as()
            .context("Could not resolve `send_stdout_as` configuration")?;
    }

    if has_python_operator {
        check_python_runtime()?;
    }

    Ok(())
}

fn check_input(
    input: &Input,
    nodes: &[super::ResolvedNode],
    input_id_str: &str,
) -> Result<(), eyre::ErrReport> {
    match &input.mapping {
        InputMapping::Timer { interval: _ } => {}
        InputMapping::User(UserInputMapping { source, output }) => {
            let source_node = nodes.iter().find(|n| &n.id == source).ok_or_else(|| {
                eyre!("source node `{source}` mapped to input `{input_id_str}` does not exist",)
            })?;
            match &source_node.kind {
                CoreNodeKind::Custom(custom_node) => {
                    if !custom_node.run_config.outputs.contains(output) {
                        bail!(
                            "output `{source}/{output}` mapped to \
                            input `{input_id_str}` does not exist",
                        );
                    }
                }
                CoreNodeKind::Runtime(runtime) => {
                    let (operator_id, output) = output.split_once('/').unwrap_or_default();
                    let operator_id = OperatorId::from(operator_id.to_owned());
                    let output = DataId::from(output.to_owned());

                    let operator = runtime
                        .operators
                        .iter()
                        .find(|o| o.id == operator_id)
                        .ok_or_else(|| {
                            eyre!(
                                "source operator `{source}/{operator_id}` used \
                                for input `{input_id_str}` does not exist",
                            )
                        })?;

                    if !operator.config.outputs.contains(&output) {
                        bail!(
                            "output `{source}/{operator_id}/{output}` mapped to \
                            input `{input_id_str}` does not exist",
                        );
                    }
                }
            }
        }
    };
    Ok(())
}

fn check_python_runtime() -> eyre::Result<()> {
    // Check if python dora-rs is installed and match cli version
    let reinstall_command =
        format!("Please reinstall it with: `pip install dora-rs=={VERSION} --force`");
    let mut command = Command::new(get_python_path().context("Could not get python binary")?);
    command.args([
        "-c",
        &format!(
            "
import dora;
assert dora.__version__=='{VERSION}',  'Python dora-rs should be {VERSION}, but current version is %s. {reinstall_command}' % (dora.__version__)
        "
        ),
    ]);
    let mut result = command
        .spawn()
        .wrap_err("Could not spawn python dora-rs command.")?;
    let status = result
        .wait()
        .wrap_err("Could not get exit status when checking python dora-rs")?;

    if !status.success() {
        bail!("Something went wrong with Python dora-rs. {reinstall_command}")
    }

    Ok(())
}
