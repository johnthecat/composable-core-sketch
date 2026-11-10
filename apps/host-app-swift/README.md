# host-app-swift

A Swift host app. It loads the library of [host-app-native](../host-app-native/README.md) and plays a product with
[product-stand-in](../product-stand-in/README.md). `make swift` builds and runs it.

## Look at

1. [`Sources/HostAppSwift/main.swift`](Sources/HostAppSwift/main.swift): the web host app's composition in Swift. The
   stand-in product drives it.
2. [`Package.swift`](Package.swift): the bindings as Swift modules.

## Tests

The app checks two things as it runs, and exits with a failure if either fails: that the product received the current
account list and then the pushed change, and that a stopped subscription receives no item when another account is added.
[docs/development.md](../../docs/development.md#tests) explains why the stop check needs no timer.
