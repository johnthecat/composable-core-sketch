# host-macros

The proc macros that write the Rust a code generator would otherwise write. They give the protocol its method tables and
give each extension point its UniFFI export and web glue, so no Rust is generated. [`src/lib.rs`](src/lib.rs) lists
which crate uses which macro.

## Look at

1. [`src/method_table.rs`](src/method_table.rs): the method table `#[wire_trait]` writes from the trait.
2. [`src/extension_point.rs`](src/extension_point.rs) and [`src/web_glue.rs`](src/web_glue.rs): the UniFFI export and
   the web glue the attribute writes.
3. [`src/extension_type.rs`](src/extension_type.rs): the derives a record or enum needs to cross an extension point, and
   `#[protocol_record]` for a protocol record an extension point also carries.
4. [`src/constructor.rs`](src/constructor.rs): what `#[host_runtime::constructor]` exports on each platform.

## Next

- [host-extension](../host-extension/README.md), the crate the extension attributes serve.
