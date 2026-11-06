//! The extension points of an interface crate, as the emitters see them.

/// Every extension point trait of an interface crate and the types they name.
#[derive(Debug)]
pub struct Extensions {
	/// Rust name of the interface crate, such as `sketch_wallet_extension`.
	pub crate_name: String,
	/// Extension point traits, in source order.
	pub traits: Vec<ExtensionTrait>,
	/// Structs and enums the traits name, each after the types it names.
	pub types: Vec<TypeDef>,
}

impl Extensions {
	/// Error types the trait methods declare, sorted and without repeats.
	pub fn error_types(&self) -> Vec<&str> {
		let mut errors: Vec<&str> = self
			.traits
			.iter()
			.flat_map(|extension| &extension.methods)
			.map(Method::error)
			.collect();
		errors.sort_unstable();
		errors.dedup();
		errors
	}
}

/// An object-safe trait the core takes as `Arc<dyn Trait>`.
#[derive(Debug)]
pub struct ExtensionTrait {
	/// Trait name, such as `Wallet`.
	pub name: String,
	/// Rustdoc comment on the trait.
	pub docs: Option<String>,
	/// Methods in declaration order.
	pub methods: Vec<Method>,
}

/// A method of an extension point trait.
#[derive(Debug)]
pub struct Method {
	/// Rust name, in snake case.
	pub name: String,
	/// Rustdoc comment on the method.
	pub docs: Option<String>,
	/// Parameters after `&self`, each taken by value.
	pub params: Vec<Param>,
	/// What the method returns.
	pub kind: MethodKind,
}

impl Method {
	/// Name of the error type the method declares.
	pub fn error(&self) -> &str {
		match &self.kind {
			MethodKind::Async { error, .. } | MethodKind::Stream { error, .. } => error,
		}
	}
}

/// The two method shapes an extension point supports.
#[derive(Debug)]
pub enum MethodKind {
	/// `async fn(..) -> Result<ok, error>`, through `async_trait`.
	Async {
		/// Value on success.
		ok: TypeRef,
		/// Name of the error type.
		error: String,
	},
	/// `fn(..) -> BoxStream<'static, Result<item, error>>`. An error ends the
	/// stream.
	Stream {
		/// Each item on success.
		item: TypeRef,
		/// Name of the error type.
		error: String,
	},
}

/// A method parameter.
#[derive(Debug)]
pub struct Param {
	/// Rust name, in snake case.
	pub name: String,
	/// Parameter type.
	pub type_ref: TypeRef,
}

/// A type a signature or a field names. Every one has a SCALE encoding.
#[derive(Debug, PartialEq, Eq)]
pub enum TypeRef {
	/// `()`.
	Unit,
	/// `bool`.
	Bool,
	/// `u8`.
	U8,
	/// `u16`.
	U16,
	/// `u32`.
	U32,
	/// `u64`.
	U64,
	/// `String`.
	String,
	/// `Vec<u8>`.
	Bytes,
	/// `[u8; N]`.
	FixedBytes(usize),
	/// `Vec<T>` for any `T` but `u8`.
	Vec(Box<TypeRef>),
	/// `Option<T>`.
	Option(Box<TypeRef>),
	/// A struct or enum of the interface crate.
	Named(String),
}

/// A struct or enum of the interface crate.
#[derive(Debug)]
pub struct TypeDef {
	/// Type name.
	pub name: String,
	/// Rustdoc comment on the type.
	pub docs: Option<String>,
	/// Struct fields or enum variants.
	pub kind: TypeDefKind,
}

/// Body of a [`TypeDef`].
#[derive(Debug)]
pub enum TypeDefKind {
	/// A struct with named fields.
	Struct(Vec<Field>),
	/// An enum whose variants are unit or have named fields.
	Enum(Vec<Variant>),
}

/// A named field.
#[derive(Debug)]
pub struct Field {
	/// Rust name, in snake case.
	pub name: String,
	/// Rustdoc comment on the field.
	pub docs: Option<String>,
	/// Field type.
	pub type_ref: TypeRef,
}

/// An enum variant. A unit variant has no fields.
#[derive(Debug)]
pub struct Variant {
	/// Variant name.
	pub name: String,
	/// Rustdoc comment on the variant.
	pub docs: Option<String>,
	/// Named fields, empty for a unit variant.
	pub fields: Vec<Field>,
}

/// `snake_case` to `camelCase`.
pub fn camel_case(snake: &str) -> String {
	let mut words = snake.split('_');
	let mut camel = words.next().unwrap_or_default().to_string();
	for word in words {
		let mut chars = word.chars();
		if let Some(first) = chars.next() {
			camel.extend(first.to_uppercase());
			camel.push_str(chars.as_str());
		}
	}
	camel
}
