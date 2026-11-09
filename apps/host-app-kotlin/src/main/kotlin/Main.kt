// A Kotlin host on the JVM: the core and both providers linked into one
// library by the host's own crate, `apps/host-app-native`. The app
// implements storage and confirmation, wraps the wallet provider in Kotlin to
// see every request the core sends to it, and plays a product by sending
// protocol frames.
//
// A native app has no TypeScript client, so the product's frames come from the
// host crate's frame builders (`signRawFrame`, `describeFrame`, ...): built from the
// protocol crate's types, they are the bytes a product's client sends.

import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import kotlin.system.exitProcess
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import uniffi.host_extension.AccountsListener
import uniffi.host_extension.Notifier
import uniffi.host_extension.ProductStorage
import uniffi.host_extension.SignRequest
import uniffi.host_extension.Signature
import uniffi.host_extension.UserConfirmation
import uniffi.host_extension.UserConfirmationReview
import uniffi.host_extension.Wallet
import uniffi.host_extension.WalletException
import uniffi.host_app_native.accountsSubscribeFrame
import uniffi.host_app_native.accountsUnsubscribeFrame
import uniffi.host_app_native.describeFrame
import uniffi.host_app_native.getAccountsFrame
import uniffi.host_app_native.hostRuntime
import uniffi.host_app_native.pushNotificationFrame
import uniffi.host_app_native.signRawFrame
import uniffi.host_app_native.storageReadFrame
import uniffi.host_app_native.storageWriteFrame
import uniffi.host_runtime.FrameSink
import uniffi.host_runtime.ProductContext
import uniffi.host_runtime.ProductRuntime
import uniffi.host_provider_notifier_log.logNotifier
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

/** Prints each frame the core sends to the product, and counts subscription
 * items. */
class PrintingSink : FrameSink {
    /** How many subscription items the product received. */
    val items = AtomicInteger()

    override fun emitFrame(frame: ByteArray) {
        val received = describeFrame(frame)
        if (received.leg == "item") items.incrementAndGet()
        println("  [product] <- ${received.leg} for ${received.requestId}: ${received.outcome}")
    }
}

/** Stops the app with a failure unless [holds], so `make kotlin` fails too. */
fun check(holds: Boolean, claim: String) {
    println("[check] ${if (holds) "ok" else "FAILED"}: $claim")
    if (!holds) exitProcess(1)
}

suspend fun send(product: ProductRuntime, label: String, frame: ByteArray) {
    println("[product] -> $label")
    product.receiveFrame(frame)
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
    val productSink = PrintingSink()
    val product = runtime.productRuntime(ProductContext("kotlin-product"), productSink)
    val alice = phone.wallet().accounts().first().publicKey
    val message = "hello from Kotlin".encodeToByteArray()

    send(product, "account.getAccounts", getAccountsFrame("1"))
    send(product, "signing.signRaw, confirmed", signRawFrame("2", alice, message))
    confirmation.approves = false
    send(product, "signing.signRaw, denied by the user", signRawFrame("3", alice, message))
    confirmation.approves = true
    phone.setDecision(PhoneDecision.DECLINE)
    send(product, "signing.signRaw, declined on the phone", signRawFrame("4", alice, message))
    phone.setDecision(PhoneDecision.APPROVE)
    send(product, "localStorage.write", storageWriteFrame("5", "note", "a note".encodeToByteArray()))
    send(product, "localStorage.read", storageReadFrame("6", "note"))
    send(product, "notifications.pushNotification", pushNotificationFrame("7", "saved"))
    send(product, "account.accountsSubscribe", accountsSubscribeFrame("8"))
    delay(100)
    println("[phone] add account bob")
    phone.addAccount("bob")
    delay(100)
    check(productSink.items.get() == 2, "the product received the current list, then the pushed change")
    send(product, "stop account.accountsSubscribe", accountsUnsubscribeFrame("8"))
    delay(100)
    println("[phone] add account carol, after the product stopped following")
    phone.addAccount("carol")
    delay(100)
    check(productSink.items.get() == 2, "a stopped subscription receives nothing more")

    println("[host] a runtime with no providers attached")
    val bare = hostRuntime(MemoryStorage(), confirmation, null, null)
        .productRuntime(ProductContext("kotlin-product"), PrintingSink())
    send(bare, "account.getAccounts", getAccountsFrame("1"))
    send(bare, "notifications.pushNotification", pushNotificationFrame("2", "saved"))
}
