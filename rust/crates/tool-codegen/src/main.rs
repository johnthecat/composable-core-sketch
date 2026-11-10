//! Generates the extension point interfaces of `@sketch/host-extension` from
//! the interface crate's rustdoc JSON. The protocol crate's rustdoc JSON
//! supplies the protocol types an extension point names.
//!
//! The Rust side needs no generated code: `#[extension_point]` writes the web
//! glue and `#[wire_trait]` the protocol's method tables.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use clap::Parser;

mod extension;
mod model;
mod rustdoc;
mod ts;

#[derive(Parser)]
#[command(about = "Generate the extension point interfaces")]
struct Cli {
	/// Rustdoc JSON of the protocol crate.
	#[arg(long)]
	protocol_input: String,

	/// Rustdoc JSON of the interface crate.
	#[arg(long)]
	extension_input: String,

	/// `src/generated` of `@sketch/host-extension`.
	#[arg(long)]
	output: String,
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	let protocol_crate = load(&cli.protocol_input)?;
	let extension_crate = load(&cli.extension_input)?;
	let world = rustdoc::World::new(vec![&protocol_crate, &extension_crate]);

	let extensions = extension::extension_traits(&world, &extension_crate)
		.context("reading the extension points")?;
	let refs: Vec<model::TypeRef> = extensions
		.iter()
		.flat_map(|extension| &extension.methods)
		.flat_map(|method| method.type_refs())
		.chain(extension::root_types(&extension_crate)?)
		.collect();
	let types = world.reachable_types(&refs)?;
	write(
		Path::new(&cli.output).join("extensions.ts"),
		ts::extensions::extensions(&extensions, &types),
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
