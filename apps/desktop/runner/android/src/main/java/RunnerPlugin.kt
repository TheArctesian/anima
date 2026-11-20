package ing.anima.runner

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@TauriPlugin
class RunnerPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun readCapabilities(invoke: Invoke) {
        activity.runOnUiThread {
            try {
                invoke.resolve(RunnerReport.read(activity))
            } catch (_: Exception) {
                invoke.reject("The Android runner report is unavailable.")
            }
        }
    }
}
