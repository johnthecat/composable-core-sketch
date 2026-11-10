# product-native

The stand-in product of the Swift and Kotlin apps. A native truapi host runs products in web views with the TS client.
The Swift and Kotlin apps have none, so they build request frames here from the protocol crate's own types
(`sign_raw_frame`, ...) and read received frames as text (`describe_frame`). They wait for each response before the next
step. The bytes are what the TS client sends. It is a test helper, not a product SDK.

[host-app-native](../host-app-native/README.md) links it like a provider, so its exports reach the apps from the same
library.

## Look at

1. [`src/lib.rs`](src/lib.rs): the frame builders, and `describe_frame`.

## Next

- [host-app-swift](../host-app-swift/README.md) and [host-app-kotlin](../host-app-kotlin/README.md), which drive it.
