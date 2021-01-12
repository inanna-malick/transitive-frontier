#[macro_use]
extern crate horrorshow;

use guppy::graph::{DependencyDirection, PackageGraph, PackageMetadata};
use guppy::MetadataCommand;
use horrorshow::helper::doctype;
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
            let dependency_source = display_name(link.from());

            if opt.debug {
                let typ = if link.to().id() == package_id {
                    "direct"
                } else {
                    "indirect"
                };
                eprintln!("\t*{}: {} -> {}", typ, &dependency_source, link.to().name());
            };

            let entry = frontier.entry(dependency_source).or_insert_with(Vec::new);
            entry.push(format!("{} {}", link.to().name(), link.to().version()))
        }
    }

    let target_dependency = {
        let meta = package_graph.metadata(package_id)?;
        format!("{} {}", meta.name(), meta.version())
    };

    let out = Output {
        target_dependency,
        frontier,
    };

    let out = match opt.format {
        OutputFmt::JSON => serde_json::to_string(&out)?,
        OutputFmt::TOML => toml::to_string(&out)?,
        OutputFmt::HTML => out.to_html(),
    };

    println!("{}", out);

    Ok(())
}

// impose kebab-case on crate names for display (standard format)
fn display_name(m: PackageMetadata) -> String {
    m.name().replace("_", "-")
}

// TODO/FIXME: this is an output format, but still: less 'String' types
#[derive(Serialize)]
struct Output {
    // dependency for which a reverse transitive dependency graph was computed
    target_dependency: String,
    /// Map of package name to list of dependencies via which a transitive dep on 'package_id' is introduced to said package
    frontier: HashMap<String, Vec<String>>,
}

impl Output {
    fn to_html(self) -> String {
        let my_title: String = format!(
            "workspace frontier for transitive dependencies on {}",
            self.target_dependency
        );
        format!(
            "{}",
            html! {
                : doctype::HTML;
                html {
                    head {
                        title : &my_title;
                    }
                    body {
                        h1(id="heading", class="title") : &my_title;
                        p {
                            : "TODO: short explainer/defn, copy from blog post";
                        }
                        ol(id="main") {
                            @ for (k,v) in self.frontier.iter() {
                                li(class="item") {
                                    : format_args!("package `{}` introduces transitive dependencies on `{}` via:", k, &self.target_dependency);
                                    ol(class="nested") {
                                        @ for dep in v.iter() {
                                            li(class="nested-item") {
                                                : format_args!("dependency: `{}`", dep)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        )
    }
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
    HTML,
}

impl FromStr for OutputFmt {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "toml" => Ok(Self::TOML),
            "json" => Ok(Self::JSON),
            "html" => Ok(Self::HTML),
            _ => Err("must be one of [toml, json]"),
        }
    }
}
