package ing.anima.accountsession

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
internal class Entry {
    lateinit var scope: String
    lateinit var account: String
}

@InvokeArg
internal class Write {
    lateinit var scope: String
    lateinit var account: String
    lateinit var token: String
}

@TauriPlugin
class AccountSessionPlugin(activity: Activity) : Plugin(activity) {
    private val tokens = SecureTokens(activity.applicationContext)

    @Command
    fun readToken(invoke: Invoke) = answer(invoke) {
        val args = invoke.parseArgs(Entry::class.java)
        JSObject().also { result ->
            tokens.read(args.scope, args.account)?.let { result.put("token", it) }
        }
    }

    @Command
    fun writeToken(invoke: Invoke) = answer(invoke) {
        val args = invoke.parseArgs(Write::class.java)
        tokens.write(args.scope, args.account, args.token)
        null
    }

    @Command
    fun clearToken(invoke: Invoke) = answer(invoke) {
        val args = invoke.parseArgs(Entry::class.java)
        tokens.clear(args.scope, args.account)
        null
    }

    private fun answer(invoke: Invoke, run: () -> JSObject?) {
        try {
            val result = run()
            if (result == null) invoke.resolve() else invoke.resolve(result)
        } catch (_: Exception) {
            invoke.reject("keystore")
        }
    }
}
