# host-app-kotlin

A Kotlin host app on the JVM. It loads the library of [host-app-native](../host-app-native/README.md) and plays a
product with [product-native](../product-native/README.md). `make kotlin` builds and runs it.

## Look at

1. [`src/main/kotlin/Main.kt`](src/main/kotlin/Main.kt): the web host app's composition in Kotlin. The stand-in product
   drives it.
2. [`build.gradle.kts`](build.gradle.kts): how the app loads the library through JNA.

## Tests

The app checks the same two things as [the Swift app](../host-app-swift/README.md#tests) as it runs.
