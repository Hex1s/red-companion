package app.red_companion

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class InstallArgs {
    lateinit var path: String
}

/** Hands a downloaded APK to the system installer (ROADMAP 8.6). Commands are called from `updater_mobile.rs`. */
@TauriPlugin
class ApkInstallerPlugin(private val activity: Activity) : Plugin(activity) {
    /** Whether the user has allowed this app to install packages ("install unknown apps"). */
    @Command
    fun canInstall(invoke: Invoke) {
        val result = JSObject()
        result.put("allowed", activity.packageManager.canRequestPackageInstalls())
        invoke.resolve(result)
    }

    /** Opens the settings screen where that permission is switched on for this app. */
    @Command
    fun openInstallSettings(invoke: Invoke) {
        val intent = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + activity.packageName))
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        activity.startActivity(intent)
        invoke.resolve()
    }

    /** Opens the system install screen for the APK at `path` (a file under the cache folder). */
    @Command
    fun install(invoke: Invoke) {
        val args = invoke.parseArgs(InstallArgs::class.java)
        val uri = FileProvider.getUriForFile(activity, activity.packageName + ".fileprovider", File(args.path))
        val intent = Intent(Intent.ACTION_VIEW)
        intent.setDataAndType(uri, "application/vnd.android.package-archive")
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        activity.startActivity(intent)
        invoke.resolve()
    }
}
