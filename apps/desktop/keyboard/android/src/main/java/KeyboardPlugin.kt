package ing.anima.keyboard

import android.app.Activity
import android.view.View
import android.webkit.WebView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Plugin

/**
 * Makes the web view end where the on-screen keyboard starts. The app draws edge to edge, so Android does not do it.
 * The page then sees a shorter window and keeps what has the focus in view (MOB-011, mobile plan M4-1).
 *
 * The root element has `data-keyboard` while the keyboard is open. The web view gives the page no inset for the
 * gesture bar, so the page keeps a fixed room for it; the keyboard covers the gesture bar, and the page then drops it.
 */
@TauriPlugin
class KeyboardPlugin(private val activity: Activity) : Plugin(activity) {
  private var open = false

  override fun load(webView: WebView) {
    val content = activity.findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      view.setPadding(0, 0, 0, keyboard)
      if (open != keyboard > 0) {
        open = keyboard > 0
        webView.evaluateJavascript("document.documentElement.toggleAttribute('data-keyboard', $open)", null)
      }
      insets
    }
    ViewCompat.requestApplyInsets(content)
  }
}
