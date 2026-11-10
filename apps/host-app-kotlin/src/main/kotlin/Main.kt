// A Kotlin host on the JVM: the core and both providers linked into one
// library by the host's own crate, `apps/host-app-native`. The app
// implements storage and confirmation. It wraps the wallet provider in Kotlin
// to see every request the core sends to it, and plays a product by sending
// protocol frames.
//
// A native app has no TypeScript client, so the product's frames come from the
// stand-in product's frame builders (`signRawFrame`, `describeFrame`, ...): built
// from the protocol crate's types, they are the bytes a product's client sends.

import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.withLock
import kotlin.system.exitProcess
import kotlinx.coroutines.runBlocking
import uniffi.host_app_native.hostRuntime
import uniffi.host_extension.AccountsListener
import uniffi.host_extension.FrameSink
import uniffi.host_extension.Notifier
import uniffi.host_extension.ProductContext
import uniffi.host_extension.ProductStorage
import uniffi.host_extension.SignRequest
import uniffi.host_extension.Signature
import uniffi.host_extension.UserConfirmation
import uniffi.host_extension.UserConfirmationReview
import uniffi.host_extension.Wallet
import uniffi.host_extension.WalletException
import uniffi.host_runtime.ProductRuntime
import uniffi.host_provider_notifier_log.logNotifier
import uniffi.product_native.FrameLeg
import uniffi.product_native.accountsSubscribeFrame
import uniffi.product_native.accountsUnsubscribeFrame
import uniffi.product_native.describeFrame
import uniffi.product_native.getAccountsFrame
import uniffi.product_native.pushNotificationFrame
import uniffi.product_native.signRawFrame
import uniffi.product_native.storageReadFrame
import uniffi.product_native.storageWriteFrame
import uniffi.protocol.ProductAccount
import uniffi.host_provider_wallet_mock.MockPhone
import uniffi.host_provider_wallet_mock.PhoneDecision

/** The app's product storage, in memory. */
class MemoryStorage : ProductStorage {
    private val values = ConcurrentHashMap<String, ByteArray>()

    override suspend fun read(key: String): ByteArray? = values[key]

    override suspend fun write(key: String, value: ByteArray) {
        values[key] = value
    }
}

/** Answers each confirmation as `approves` says. */
class ScriptedConfirmation : UserConfirmation {
    @Volatile var approves = true

    override suspend fun confirmUserAction(review: UserConfirmationReview): Boolean {
        if (review is UserConfirmationReview.SignRaw) {
            val message = review.payload.decodeToString()
            println("  [confirmation] ${review.productId} asks to sign \"$message\": ${if (approves) "approved" else "denied"}")
        }
        return approves
    }
}

/** Kotlin code around the wallet provider: it forwards every call, so it sees
 * each request the core sends and how it ends, and it gives the provider its
 * own accounts listener, so it sees each change the provider pushes. */
class TrackingWallet(private val inner: Wallet) : Wallet {
    override suspend fun accounts(): List<ProductAccount> = track("accounts") { inner.accounts() }

    override suspend fun sign(request: SignRequest): Signature =
        track("sign ${request.payload.size} bytes") { inner.sign(request) }

    override fun setAccountsListener(listener: AccountsListener) {
        inner.setAccountsListener(TrackingListener(listener))
    }

    private suspend fun <T> track(request: String, call: suspend () -> T): T {
        println("  [wallet] $request: pending on the phone")
        try {
            return call().also { println("  [wallet] $request: answered") }
        } catch (error: WalletException) {
            println("  [wallet] $request: ${error::class.simpleName}")
            throw error
        }
    }
}

/** Prints each account list the wallet pushes, then passes it to the core. */
class TrackingListener(private val core: AccountsListener) : AccountsListener {
    override fun accountsChanged(accounts: List<ProductAccount>) {
        println("  [wallet] pushed: ${accounts.map { it.name }}")
        core.accountsChanged(accounts)
    }
}

/** The RFC's example: Kotlin code around the log notifier that also shows
 * each notice in a status bar. */
class StatusNotifier(private val inner: Notifier) : Notifier {
    override suspend fun notify(text: String) {
        println("  [status bar] $text")
        inner.notify(text)
    }
}

/** Prints each frame the core sends to the product, and counts them, so the
 * app waits for a frame instead of sleeping. */
class PrintingSink : FrameSink {
    private val lock = ReentrantLock()
    private val arrived = lock.newCondition()
    private var frames = 0
    private var itemCount = 0

    /** How many subscription items the product received. */
    val items: Int get() = lock.withLock { itemCount }

    override fun emitFrame(frame: ByteArray) {
        val received = describeFrame(frame)
        println("  [product] <- ${received.leg} for ${received.requestId}: ${received.outcome}")
        lock.withLock {
            frames += 1
            if (received.leg == FrameLeg.ITEM) itemCount += 1
            arrived.signalAll()
        }
    }

    /** Waits until the product has received [count] frames in all. */
    fun waitForFrames(count: Int) {
        lock.withLock {
            while (frames < count) {
                if (!arrived.await(5, TimeUnit.SECONDS)) check(false, "frame $count arrived")
            }
        }
    }
}

/** Stops the app with a failure unless [holds], so `make kotlin` fails too. */
fun check(holds: Boolean, claim: String) {
    println("[check] ${if (holds) "ok" else "FAILED"}: $claim")
    if (!holds) exitProcess(1)
}

/** Sends a product frame, then waits until the product has received
 * [expecting] frames in all. */
fun send(product: ProductRuntime, sink: PrintingSink, label: String, frame: ByteArray, expecting: Int) {
    println("[product] -> $label")
    product.receiveFrame(frame)
    sink.waitForFrames(expecting)
}

fun main() = runBlocking {
    val phone = MockPhone(200u)
    phone.addAccount("alice")
    val confirmation = ScriptedConfirmation()
    val runtime = hostRuntime(
        MemoryStorage(),
        confirmation,
        TrackingWallet(phone.wallet()),
        StatusNotifier(logNotifier()),
    )
    val sink = PrintingSink()
    val product = runtime.productRuntime(ProductContext("kotlin-product"), sink)
    val alice = phone.wallet().accounts().first().publicKey
    val message = "hello from Kotlin".encodeToByteArray()

    send(product, sink, "account.getAccounts", getAccountsFrame("1"), 1)
    send(product, sink, "signing.signRaw, confirmed", signRawFrame("2", alice, message), 2)
    confirmation.approves = false
    send(product, sink, "signing.signRaw, denied by the user", signRawFrame("3", alice, message), 3)
    confirmation.approves = true
    phone.setDecision(PhoneDecision.DECLINE)
    send(product, sink, "signing.signRaw, declined on the phone", signRawFrame("4", alice, message), 4)
    phone.setDecision(PhoneDecision.APPROVE)
    send(product, sink, "localStorage.write", storageWriteFrame("5", "note", "a note".encodeToByteArray()), 5)
    send(product, sink, "localStorage.read", storageReadFrame("6", "note"), 6)
    send(product, sink, "notifications.pushNotification", pushNotificationFrame("7", "saved"), 7)
    send(product, sink, "account.accountsSubscribe", accountsSubscribeFrame("8"), 8)
    println("[phone] add account bob")
    phone.addAccount("bob")
    sink.waitForFrames(9)
    check(sink.items == 2, "the product received the current list, then the pushed change")
    // No timing: `receiveFrame` serves the stop before it returns, and the
    // core's one executor runs calls in the order they are woken, so an item for
    // carol would reach the sink before the answer to the next call.
    send(product, sink, "stop account.accountsSubscribe", accountsUnsubscribeFrame("8"), 9)
    println("[phone] add account carol, after the product stopped following")
    phone.addAccount("carol")
    send(product, sink, "account.getAccounts, which sees carol", getAccountsFrame("9"), 10)
    check(sink.items == 2, "a stopped subscription receives nothing more")

    println("[host] a runtime with no providers attached")
    val bareSink = PrintingSink()
    val bare = hostRuntime(MemoryStorage(), confirmation, null, null)
        .productRuntime(ProductContext("kotlin-product"), bareSink)
    send(bare, bareSink, "account.getAccounts", getAccountsFrame("1"), 1)
    send(bare, bareSink, "notifications.pushNotification", pushNotificationFrame("2", "saved"), 2)
}
