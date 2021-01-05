# transitive-frontier

I’ve been meaning to find an excuse to build something using guppy, and decided on a simple tool to audit the places where some dependency finds its way into a cargo workspace, either directly or transitively. I’ve called this tool `transitive-frontier`, because it finds the places along the workspace frontier where transitive dependencies on some crate are introduced.

The use case I had in mind was large projects moving from futures 0.1 to futures 0.3. In practice, it’s usually not that hard to write up a list of workspace crates that need to be refactored to not use futures 0.1, but a machine-readable report opens up new possibilities - for example, you could use the output of this tool to build a linter that asserts that no new transitive dependencies on futures 0.1 have been introduced into a workspace. This project has also been a great way to familiarize myself with guppy.

