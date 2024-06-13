use dora_node_api_c::HEADER_NODE_API;
use dora_operator_api_c::{HEADER_OPERATOR_API, HEADER_OPERATOR_TYPES};
use eyre::{bail, Context};
use std::{
    fs,
    path::{Path, PathBuf},
};

const NODE: &str = include_str!("node/node-template.c");
const TALKER: &str = include_str!("talker/talker-template.c");
const LISTENER: &str = include_str!("listener/listener-template.c");

pub fn create(args: crate::CommandNew, use_path_deps: bool) -> eyre::Result<()> {
    let crate::CommandNew {
        kind,
        lang: _,
        name,
        path,
    } = args;

    match kind {
        crate::Kind::Operator => {
            bail!("Operators are going to be depreciated, please don't use it")
        }
        crate::Kind::CustomNode => create_custom_node(name, path, NODE),
        crate::Kind::Dataflow => create_dataflow(name, path, use_path_deps),
    }
}

fn create_dataflow(
    name: String,
    path: Option<PathBuf>,
    use_path_deps: bool,
) -> Result<(), eyre::ErrReport> {
    const DATAFLOW_YML: &str = include_str!("dataflow-template.yml");

    if name.contains('/') {
        bail!("dataflow name must not contain `/` separators");
    }
    if !name.is_ascii() {
        bail!("dataflow name must be ASCII");
    }

    // create directories
    let root = path.as_deref().unwrap_or_else(|| Path::new(&name));
    fs::create_dir(root)
        .with_context(|| format!("failed to create directory `{}`", root.display()))?;

    let dataflow_yml = DATAFLOW_YML.replace("___name___", &name);
    let dataflow_yml_path = root.join("dataflow.yml");
    fs::write(&dataflow_yml_path, dataflow_yml)
        .with_context(|| format!("failed to write `{}`", dataflow_yml_path.display()))?;

    create_custom_node("talker_1".into(), Some(root.join("talker_1")), TALKER)?;
    create_custom_node("talker_2".into(), Some(root.join("talker_2")), TALKER)?;
    create_custom_node("listener_1".into(), Some(root.join("listener_1")), LISTENER)?;
    create_cmakefile(root.to_path_buf(), use_path_deps)?;

    println!(
        "Created new C dataflow at `{name}` at {}",
        Path::new(".").join(root).display()
    );

    Ok(())
}

#[deprecated(since = "0.3.4")]
#[allow(unused)]
fn create_operator(name: String, path: Option<PathBuf>) -> Result<(), eyre::ErrReport> {
    const OPERATOR: &str = include_str!("operator/operator-template.c");

    if name.contains('/') {
        bail!("operator name must not contain `/` separators");
    }
    if name.contains('-') {
        bail!("operator name must not contain `-` separators");
    }
    if !name.is_ascii() {
        bail!("operator name must be ASCII");
    }

    // create directories
    let root = path.as_deref().unwrap_or_else(|| Path::new(&name));
    fs::create_dir(root)
        .with_context(|| format!("failed to create directory `{}`", root.display()))?;

    let operator_path = root.join("operator.c");
    fs::write(&operator_path, OPERATOR)
        .with_context(|| format!("failed to write `{}`", operator_path.display()))?;
    let header_api_path = root.join("operator_api.h");
    let header_type_path = root.join("operator_types.h");
    fs::write(&header_api_path, HEADER_OPERATOR_API)
        .with_context(|| format!("failed to write `{}`", header_api_path.display()))?;
    fs::write(&header_type_path, HEADER_OPERATOR_TYPES)
        .with_context(|| format!("failed to write `{}`", header_type_path.display()))?;

    // TODO: Makefile?

    println!(
        "Created new C operator `{name}` at {}",
        Path::new(".").join(root).display()
    );

    Ok(())
}

fn create_cmakefile(root: PathBuf, use_path_deps: bool) -> Result<(), eyre::ErrReport> {
    const CMAKEFILE: &str = include_str!("cmake-template.txt");

    let cmake_file = if use_path_deps {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
        CMAKEFILE.replace("__DORA_PATH__", workspace_dir.to_str().unwrap())
    } else {
        CMAKEFILE.replace("__DORA_PATH__", "")
    };

    let cmake_path = root.join("CMakeLists.txt");
    fs::write(&cmake_path, cmake_file)
        .with_context(|| format!("failed to write `{}`", cmake_path.display()))?;

    println!("Created new CMakeLists.txt at {}", cmake_path.display());
    Ok(())
}

fn create_custom_node(
    name: String,
    path: Option<PathBuf>,
    template_scripts: &str,
) -> Result<(), eyre::ErrReport> {
    if name.contains('/') {
        bail!("node name must not contain `/` separators");
    }
    if !name.is_ascii() {
        bail!("node name must be ASCII");
    }

    // create directories
    let root = path.as_deref().unwrap_or_else(|| Path::new(&name));
    fs::create_dir(root)
        .with_context(|| format!("failed to create directory `{}`", root.display()))?;

    let node_path = root.join("node.c");
    fs::write(&node_path, template_scripts)
        .with_context(|| format!("failed to write `{}`", node_path.display()))?;
    let header_path = root.join("node_api.h");
    fs::write(&header_path, HEADER_NODE_API)
        .with_context(|| format!("failed to write `{}`", header_path.display()))?;

    // TODO: Makefile?

    println!(
        "Created new C custom node `{name}` at {}",
        Path::new(".").join(root).display()
    );

    Ok(())
}
