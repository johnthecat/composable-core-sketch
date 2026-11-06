//! Generates the web side of each extension point from the interface crate's
//! rustdoc JSON: the TypeScript interfaces and adapters for the host package, and
//! `src/wasm.rs` for the interface crate.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

mod model;
mod rust;
mod rustdoc;
mod ts;

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(about = "Generate the web side of extension points from rustdoc JSON")]
struct Cli {
	/// Rustdoc JSON of the interface crate, built with the nightly in
	/// `nightly-toolchain`.
	#[arg(long)]
	input: PathBuf,

	/// Directory that receives `extensions.ts` and `extensions-adapter.ts`.
	#[arg(long)]
	ts_output: PathBuf,

	/// Directory that receives `wasm.rs`: the interface crate's `src`.
	#[arg(long)]
	rust_output: PathBuf,
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	let input = cli.input.display();
	let json = fs::read_to_string(&cli.input).with_context(|| format!("reading {input}"))?;
	let extensions = rustdoc::parse(&json)
		.and_then(|krate| krate.extensions())
		.with_context(|| format!("extracting extension points from {input}"))?;

	fs::create_dir_all(&cli.ts_output)?;
	write(cli.ts_output.join("extensions.ts"), ts::interfaces(&extensions))?;
	write(cli.ts_output.join("extensions-adapter.ts"), ts::adapters(&extensions))?;
	write(cli.rust_output.join("wasm.rs"), rust::wasm_bridge(&extensions))?;
	Ok(())
}

fn write(path: PathBuf, contents: String) -> Result<()> {
	fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
	println!("wrote {}", path.display());
	Ok(())
}
