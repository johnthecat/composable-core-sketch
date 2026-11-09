//! Generates the TypeScript side of the sketch from rustdoc JSON: the product
//! client (`@sketch/protocol-client`) from the protocol crate, and the
//! extension point interfaces (`@sketch/host`) from the interface crate.
//!
//! The Rust side needs no generated code: `#[wire_trait]` writes the
//! protocol's method tables and `#[extension_point]` the web glue.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use clap::Parser;

mod extension;
mod model;
mod protocol;
mod rustdoc;
mod ts;

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(about = "Generate the product client and the extension point interfaces")]
struct Cli {
	/// Rustdoc JSON of the protocol crate.
	#[arg(long)]
	protocol_input: String,

	/// Rustdoc JSON of the interface crate.
	#[arg(long)]
	extension_input: String,

	/// `src/generated` of `@sketch/protocol-client`.
	#[arg(long)]
	client_output: String,

	/// `src/generated` of `@sketch/host`.
	#[arg(long)]
	host_output: String,
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	let protocol_crate = load(&cli.protocol_input)?;
	let extension_crate = load(&cli.extension_input)?;
	let world = rustdoc::World::new(vec![&protocol_crate, &extension_crate]);

	let api = protocol::extract(&world, &protocol_crate).context("reading the protocol")?;
	let api_types = world.reachable_types(
		api.iter().flat_map(|api| &api.methods).flat_map(|method| method.type_refs()),
	)?;
	let client = Path::new(&cli.client_output);
	write(client.join("types.ts"), ts::client::types(&api_types))?;
	write(client.join("wire-table.ts"), ts::client::wire_table(&api))?;
	write(client.join("client.ts"), ts::client::client(&api, &api_types))?;

	let extensions = extension::extension_traits(&world, &extension_crate)
		.context("reading the extension points")?;
	let refs: Vec<model::TypeRef> = extensions
		.iter()
		.flat_map(|extension| &extension.methods)
		.flat_map(|method| method.type_refs())
		.collect();
	let types = world.reachable_types(&refs)?;
	write(
		Path::new(&cli.host_output).join("extensions.ts"),
		ts::extensions::extensions(extension_crate.name()?, &extensions, &types),
	)?;
	Ok(())
}

fn load(path: &str) -> Result<rustdoc::Crate> {
	let json = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
	rustdoc::parse(&json).with_context(|| format!("parsing {path}"))
}

fn write(path: impl AsRef<Path>, contents: String) -> Result<()> {
	let path = path.as_ref();
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}
	fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
	println!("wrote {}", path.display());
	Ok(())
}
