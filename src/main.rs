use guppy::graph::{DependencyDirection, PackageGraph};
use guppy::MetadataCommand;
use serde::Serialize;
use std::{collections::HashMap, error::Error, iter, path::PathBuf, str::FromStr};
use structopt::StructOpt;

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let mut cmd = MetadataCommand::new();
    if let Some(workspace) = &opt.workspace {
        cmd.current_dir(workspace);
    }
    let package_graph = PackageGraph::from_command(&mut cmd)?;

    let package_id = {
        let mut candidates = Vec::new();

        for id in package_graph.package_ids() {
            if id.repr().contains(&opt.package_id) {
                candidates.push(id);
            }
        }

        if candidates.len() == 1 {
            Ok(candidates[0])
        } else {
            for id in candidates.iter() {
                eprintln!("\t - {}", id.repr())
            }
            Err(format!(
                "package-id substring should match exactly one package id, {}",
                &opt.package_id
            ))
        }
    }?;

    let package_set = package_graph
        .query_reverse(iter::once(package_id))?
        .resolve_with_fn(|_, link| !opt.skip.iter().any(|s| link.to().id().repr().contains(s)));

    if opt.debug {
        eprintln!("workspace frontier for dependencies on {}:", &package_id);
    };

    let mut frontier = HashMap::new();

    // reverse deps
    for link in package_set.links(DependencyDirection::Reverse) {
        // != implements logical xor
        if link.to().in_workspace() != link.from().in_workspace() {
            if opt.debug {
                let typ = if link.to().id() == package_id {
                    "direct"
                } else {
                    "indirect"
                };
                eprintln!("\t*{}: {} -> {}", typ, link.from().name(), link.to().name());
            };

            let entry = frontier
                .entry(link.from().name().to_string())
                .or_insert_with(Vec::new);
            entry.push(format!("{} {}", link.to().name(), link.to().version()))
        }
    }

    let out = Output {
        package_id: format!("{}", package_id.repr()),
        frontier,
    };

    let out = match opt.format {
        OutputFmt::JSON => serde_json::to_string(&out)?,
        OutputFmt::TOML => toml::to_string(&out)?,
    };

    println!("{}", out);

    Ok(())
}

// TODO/FIXME: this is an output format, but still: less 'String' types
#[derive(Serialize)]
struct Output {
    // full package id for which reverse transitive dependencies were computed
    package_id: String,
    /// Map of package name to list of dependencies via which a transitive dep on 'package_id' is introduced to said package
    frontier: HashMap<String, Vec<String>>,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "transitive_frontier",
    about = "given a package id substring and workspace root, find all graph links where dependencies on that package are introduced into the workspace"
)]
struct Opt {
    /// Activate debug logging (via stderr)
    #[structopt(short, long)]
    debug: bool,

    /// Workspace on which to run guppy. Defaults to current directory if not present.
    #[structopt(parse(from_os_str))]
    workspace: Option<PathBuf>,

    /// substring of package id to run on. must be unique in the workspace's package graph.
    #[structopt(short)]
    package_id: String,

    /// links to skip when resolving reverse transitive dependencies
    #[structopt(short, long)]
    skip: Vec<String>,

    /// output format. defaults to toml
    #[structopt(short, long, default_value = "toml")]
    format: OutputFmt,
}

#[derive(Debug)]
enum OutputFmt {
    TOML,
    JSON,
}

impl FromStr for OutputFmt {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "toml" => Ok(Self::TOML),
            "json" => Ok(Self::JSON),
            _ => Err("must be one of [toml, json]"),
        }
    }
}
