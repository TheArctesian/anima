package ing.anima.runner

import android.app.Activity
import android.app.ActivityManager
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.BatteryManager
import android.os.Build
import android.os.PowerManager
import android.os.StatFs
import android.system.Os
import android.system.OsConstants
import android.webkit.WebView
import app.tauri.plugin.JSObject
import org.json.JSONArray
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

internal object RunnerReport {
    fun read(activity: Activity): JSObject {
        val runtime = Runtime.getRuntime()
        val power = checkNotNull(activity.getSystemService(PowerManager::class.java))
        val manager = checkNotNull(activity.getSystemService(ActivityManager::class.java))
        val memory = ActivityManager.MemoryInfo().also(manager::getMemoryInfo)
        val activeCpus = runtime.availableProcessors()
        val logicalCpus = Os.sysconf(OsConstants._SC_NPROCESSORS_CONF)

        return JSObject().apply {
            put("schemaVersion", 1)
            put("platform", "android")
            put("simulated", Build.FINGERPRINT.startsWith("generic") ||
                Build.HARDWARE in setOf("goldfish", "ranchu") || Build.MODEL.contains("Emulator"))
            put("observedAt", timestamp())
            put("osVersion", Build.VERSION.RELEASE)
            put("sdkVersion", Build.VERSION.SDK_INT)
            put("logicalCPUCount", maxOf(logicalCpus, activeCpus.toLong()))
            put("activeCPUCount", activeCpus)
            put("physicalMemoryBytes", memory.totalMem)
            put("availableSystemMemoryBytes", memory.availMem)
            put("javaHeapHeadroomBytes", runtime.maxMemory() - (runtime.totalMemory() - runtime.freeMemory()))
            put("availableStorageBytes", StatFs(activity.filesDir.absolutePath).availableBytes)
            put("charging", charging(activity))
            put("thermal", thermal(power))
            put("lowPowerMode", power.isPowerSaveMode)
            put("foreground", activity.hasWindowFocus())
            put("network", network(activity))
            put("vulkan", vulkan(activity.packageManager))
            if (Build.VERSION.SDK_INT >= 26) {
                WebView.getCurrentWebViewPackage()?.versionName?.let { put("webViewVersion", it) }
            }
        }
    }

    private fun timestamp(): String =
        SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.ROOT).apply {
            timeZone = TimeZone.getTimeZone("UTC")
        }.format(Date())

    private fun charging(activity: Activity): String {
        val battery = activity.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
            ?: return "unknown"
        val plugged = battery.getIntExtra(BatteryManager.EXTRA_PLUGGED, -1)
        if (plugged == 0) return "unplugged"
        return when (battery.getIntExtra(BatteryManager.EXTRA_STATUS, BatteryManager.BATTERY_STATUS_UNKNOWN)) {
            BatteryManager.BATTERY_STATUS_CHARGING -> "charging"
            BatteryManager.BATTERY_STATUS_FULL -> if (plugged > 0) "full" else "unknown"
            BatteryManager.BATTERY_STATUS_NOT_CHARGING, BatteryManager.BATTERY_STATUS_DISCHARGING ->
                if (plugged > 0) "notCharging" else "unknown"
            else -> "unknown"
        }
    }

    private fun thermal(power: PowerManager): String {
        if (Build.VERSION.SDK_INT < 29) return "unknown"
        return when (power.currentThermalStatus) {
            PowerManager.THERMAL_STATUS_NONE -> "nominal"
            PowerManager.THERMAL_STATUS_LIGHT, PowerManager.THERMAL_STATUS_MODERATE -> "fair"
            PowerManager.THERMAL_STATUS_SEVERE -> "serious"
            PowerManager.THERMAL_STATUS_CRITICAL, PowerManager.THERMAL_STATUS_EMERGENCY,
            PowerManager.THERMAL_STATUS_SHUTDOWN -> "critical"
            else -> "unknown"
        }
    }

    private fun network(activity: Activity): JSObject {
        val manager = checkNotNull(activity.getSystemService(ConnectivityManager::class.java))
        val capabilities = manager.getNetworkCapabilities(manager.activeNetwork)
        val transports = listOf(
            NetworkCapabilities.TRANSPORT_WIFI to "wifi",
            NetworkCapabilities.TRANSPORT_CELLULAR to "cellular",
            NetworkCapabilities.TRANSPORT_ETHERNET to "wired",
            NetworkCapabilities.TRANSPORT_VPN to "vpn",
        )
        return JSObject().apply {
            put("status", if (capabilities == null) "unsatisfied" else "satisfied")
            put("interfaces", JSONArray(transports.filter { capabilities?.hasTransport(it.first) == true }.map { it.second }))
            put("expensive", capabilities != null && manager.isActiveNetworkMetered)
            put("validated", capabilities?.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED) == true)
        }
    }

    private fun vulkan(manager: PackageManager): JSObject {
        val features = manager.systemAvailableFeatures.associateBy { it.name }
        return JSObject().apply {
            features[PackageManager.FEATURE_VULKAN_HARDWARE_VERSION]?.let { feature ->
                val version = feature.version
                put("apiVersion", "${version ushr 22}.${(version ushr 12) and 0x3ff}.${version and 0xfff}")
            }
            features[PackageManager.FEATURE_VULKAN_HARDWARE_LEVEL]?.let { put("hardwareLevel", it.version) }
            if (Build.VERSION.SDK_INT >= 26) {
                features[PackageManager.FEATURE_VULKAN_HARDWARE_COMPUTE]?.let { put("computeLevel", it.version) }
            }
        }
    }
}
