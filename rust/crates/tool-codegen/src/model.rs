//! What tool-codegen extracts from rustdoc JSON, independent of the emitters.

use core::fmt;

/// A type a signature or a field names. A protocol type has a SCALE encoding;
/// an extension point value crosses as a typed JS value instead.
#[derive(Debug, Clone, PartialEq, Eq)]
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
	/// `[u8; N]`, or a type alias of one.
	FixedBytes(usize),
	/// `Vec<T>` for any `T` but `u8`.
	Vec(Box<TypeRef>),
	/// `Option<T>`.
	Option(Box<TypeRef>),
	/// A struct or enum of a loaded crate.
	Named(TypeName),
}

/// A struct or enum, by its full path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeName {
	/// Full path, starting with the defining crate.
	pub path: Vec<String>,
}

impl TypeName {
	/// The type at `path`.
	pub fn new(path: Vec<String>) -> Self {
		Self { path }
	}

	/// The defining crate.
	pub fn crate_name(&self) -> &str {
		self.path.first().map(String::as_str).unwrap_or_default()
	}

	/// The name in Rust source.
	pub fn rust_name(&self) -> &str {
		self.path.last().map(String::as_str).unwrap_or_default()
	}

	/// The name in TypeScript. A versioned envelope shares its payload's Rust
	/// name, so it gets a `Versioned` prefix there.
	pub fn ts_name(&self) -> String {
		if self.path.iter().any(|segment| segment == "versioned") {
			format!("Versioned{}", self.rust_name())
		} else {
			self.rust_name().to_string()
		}
	}

	/// The path from the crate root, `crate`-relative for a local type.
	pub fn rust_path(&self) -> String {
		self.path.join("::")
	}
}

impl fmt::Display for TypeName {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(&self.rust_path())
	}
}

/// A struct or enum definition.
pub struct TypeDef {
	/// Where it is defined.
	pub name: TypeName,
	/// Rustdoc comment.
	pub docs: Option<String>,
	/// Fields or variants.
	pub kind: TypeDefKind,
}

impl TypeDef {
	/// Every type the definition names directly.
	pub fn type_refs(&self) -> Vec<&TypeRef> {
		match &self.kind {
			TypeDefKind::Struct(fields) => fields.iter().map(|field| &field.type_ref).collect(),
			TypeDefKind::Enum(variants) => variants
				.iter()
				.flat_map(|variant| match &variant.shape {
					VariantShape::Unit => Vec::new(),
					VariantShape::Newtype(inner) => vec![inner],
					VariantShape::Named(fields) => {
						fields.iter().map(|field| &field.type_ref).collect()
					},
				})
				.collect(),
		}
	}
}

/// Body of a [`TypeDef`].
pub enum TypeDefKind {
	/// A struct with named fields.
	Struct(Vec<Field>),
	/// An enum.
	Enum(Vec<EnumVariant>),
}

/// A named field.
pub struct Field {
	/// Rust name, in snake case.
	pub name: String,
	/// Rustdoc comment.
	pub docs: Option<String>,
	/// Field type.
	pub type_ref: TypeRef,
}

/// One variant of an enum.
pub struct EnumVariant {
	/// Variant name.
	pub name: String,
	/// Rustdoc comment.
	pub docs: Option<String>,
	/// Its payload.
	pub shape: VariantShape,
}

/// Payload of an enum variant.
pub enum VariantShape {
	/// No payload.
	Unit,
	/// One unnamed field, as in a versioned envelope's `V1(payload)`.
	Newtype(TypeRef),
	/// Named fields.
	Named(Vec<Field>),
}
