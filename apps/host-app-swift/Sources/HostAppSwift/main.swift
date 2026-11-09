// A Swift host: the core and both providers linked into one library by the
// host's own crate, `apps/host-app-native`. The app implements storage
// and confirmation, wraps the wallet provider in Swift to see every request the
// core sends to it, and plays a product by sending protocol frames.
//
// A native app has no TypeScript client, so the product's frames come from the
// host crate's frame builders (`signRawFrame`, `describeFrame`, ...): built from the
// protocol crate's types, they are the bytes a product's client sends.

import Foundation
import HostAppNative

/// The app's product storage, in memory.
final class MemoryStorage: ProductStorage, @unchecked Sendable {
    private let lock = NSLock()
    private var values: [String: Data] = [:]

    func read(key: String) async throws -> Data? {
        lock.withLock { values[key] }
    }

    func write(key: String, value: Data) async throws {
        lock.withLock { values[key] = value }
    }
}

/// Answers each confirmation as `approves` says.
final class ScriptedConfirmation: UserConfirmation, @unchecked Sendable {
    var approves = true

    func confirmUserAction(review: UserConfirmationReview) async throws -> Bool {
        if case let .signRaw(productId, _, payload) = review {
            let message = String(decoding: payload, as: UTF8.self)
            print("  [confirmation] \(productId) asks to sign \"\(message)\": \(approves ? "approved" : "denied")")
        }
        return approves
    }
}

/// Swift code around the wallet provider: it forwards every call, so it sees
/// each request the core sends and how it ends, and it gives the provider its
/// own accounts listener, so it sees each change the provider pushes.
final class TrackingWallet: Wallet, @unchecked Sendable {
    private let inner: Wallet

    init(inner: Wallet) {
        self.inner = inner
    }

    func accounts() async throws -> [ProductAccount] {
        try await track("accounts") { try await self.inner.accounts() }
    }

    func sign(request: SignRequest) async throws -> Signature {
        try await track("sign \(request.payload.count) bytes") {
            try await self.inner.sign(request: request)
        }
    }

    func setAccountsListener(listener: AccountsListener) {
        inner.setAccountsListener(listener: TrackingListener(core: listener))
    }

    private func track<T>(_ request: String, _ call: () async throws -> T) async throws -> T {
        print("  [wallet] \(request): pending on the phone")
        do {
            let value = try await call()
            print("  [wallet] \(request): answered")
            return value
        } catch {
            print("  [wallet] \(request): \(error)")
            throw error
        }
    }
}

/// Prints each account list the wallet pushes, then passes it to the core.
final class TrackingListener: AccountsListener, @unchecked Sendable {
    private let core: AccountsListener

    init(core: AccountsListener) {
        self.core = core
    }

    func accountsChanged(accounts: [ProductAccount]) {
        print("  [wallet] pushed: \(accounts.map(\.name))")
        core.accountsChanged(accounts: accounts)
    }
}

/// The RFC's example: Swift code around the log notifier that also shows each
/// notice in a status bar.
final class StatusNotifier: Notifier, @unchecked Sendable {
    private let inner: Notifier

    init(inner: Notifier) {
        self.inner = inner
    }

    func notify(text: String) async throws {
        print("  [status bar] \(text)")
        try await inner.notify(text: text)
    }
}

/// Prints each frame the core sends to the product, and counts subscription
/// items.
final class PrintingSink: FrameSink, @unchecked Sendable {
    private let lock = NSLock()
    private var itemCount = 0

    /// How many subscription items the product received.
    var items: Int { lock.withLock { itemCount } }

    func emitFrame(frame: Data) {
        let received = describeFrame(frame: frame)
        if received.leg == "item" {
            lock.withLock { itemCount += 1 }
        }
        print("  [product] <- \(received.leg) for \(received.requestId): \(received.outcome)")
    }
}

/// Stops the app with a failure unless `holds`, so `make swift` fails too.
func check(_ holds: Bool, _ claim: String) {
    print("[check] \(holds ? "ok" : "FAILED"): \(claim)")
    if !holds {
        exit(1)
    }
}

func send(_ product: ProductRuntime, _ label: String, _ frame: Data) async throws {
    print("[product] -> \(label)")
    try await product.receiveFrame(frame: frame)
}

// Rust and Swift share stdout; line buffering keeps their lines in order.
setvbuf(stdout, nil, _IOLBF, 0)

let phone = MockPhone(signDelayMs: 200)
phone.addAccount(name: "alice")
let confirmation = ScriptedConfirmation()
let runtime = hostRuntime(
    productStorage: MemoryStorage(),
    userConfirmation: confirmation,
    wallet: TrackingWallet(inner: phone.wallet()),
    notifier: StatusNotifier(inner: logNotifier())
)
let productSink = PrintingSink()
let product = runtime.productRuntime(
    product: ProductContext(productId: "swift-product"),
    sink: productSink
)
let alice = try await phone.wallet().accounts()[0].publicKey
let message = Data("hello from Swift".utf8)

try await send(product, "account.getAccounts", getAccountsFrame(requestId: "1"))
try await send(product, "signing.signRaw, confirmed", signRawFrame(requestId: "2", publicKey: alice, payload: message))
confirmation.approves = false
try await send(product, "signing.signRaw, denied by the user", signRawFrame(requestId: "3", publicKey: alice, payload: message))
confirmation.approves = true
phone.setDecision(decision: .decline)
try await send(product, "signing.signRaw, declined on the phone", signRawFrame(requestId: "4", publicKey: alice, payload: message))
phone.setDecision(decision: .approve)
try await send(product, "localStorage.write", storageWriteFrame(requestId: "5", key: "note", value: Data("a note".utf8)))
try await send(product, "localStorage.read", storageReadFrame(requestId: "6", key: "note"))
try await send(product, "notifications.pushNotification", pushNotificationFrame(requestId: "7", text: "saved"))
try await send(product, "account.accountsSubscribe", accountsSubscribeFrame(requestId: "8"))
try await Task.sleep(nanoseconds: 100_000_000)
print("[phone] add account bob")
phone.addAccount(name: "bob")
try await Task.sleep(nanoseconds: 100_000_000)
check(productSink.items == 2, "the product received the current list, then the pushed change")
try await send(product, "stop account.accountsSubscribe", accountsUnsubscribeFrame(requestId: "8"))
try await Task.sleep(nanoseconds: 100_000_000)
print("[phone] add account carol, after the product stopped following")
phone.addAccount(name: "carol")
try await Task.sleep(nanoseconds: 100_000_000)
check(productSink.items == 2, "a stopped subscription receives nothing more")

print("[host] a runtime with no providers attached")
let bare = hostRuntime(
    productStorage: MemoryStorage(),
    userConfirmation: confirmation,
    wallet: nil,
    notifier: nil
).productRuntime(product: ProductContext(productId: "swift-product"), sink: PrintingSink())
try await send(bare, "account.getAccounts", getAccountsFrame(requestId: "1"))
try await send(bare, "notifications.pushNotification", pushNotificationFrame(requestId: "2", text: "saved"))
