package io.github.gunsandsalvi.phoenix

import android.app.ActivityManager
import android.content.Intent
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import java.io.File
import java.time.Instant
import kotlin.concurrent.thread
import uniffi.phx_ffi.BenchHost
import uniffi.phx_ffi.BenchLine
import uniffi.phx_ffi.DeviceInfo
import uniffi.phx_ffi.runBench
import uniffi.phx_ffi.runLoad
import uniffi.phx_ffi.runProgramme
import uniffi.phx_ffi.runWorld

private enum class Phase { READY, RUNNING, DONE }

private val THERMAL = listOf("none", "light", "moderate", "severe", "critical", "emergency", "shutdown")

/** The world's turns the bench runs from its opening: about three months of business days. */
private const val WORLD_TURNS = 60u

/** Copies an asset directory to the app's files, so the engine reads it as it reads the repository's data. */
private fun unpack(activity: ComponentActivity, asset: String, into: File) {
    val names = activity.assets.list(asset) ?: emptyArray()
    if (names.isEmpty()) {
        into.parentFile?.mkdirs()
        activity.assets.open(asset).use { input -> into.outputStream().use { input.copyTo(it) } }
        return
    }
    into.mkdirs()
    for (name in names) unpack(activity, "$asset/$name", File(into, name))
}

private fun device(activity: ComponentActivity): DeviceInfo {
    val memory = ActivityManager.MemoryInfo()
    activity.getSystemService(ActivityManager::class.java).getMemoryInfo(memory)
    val version = activity.packageManager.getPackageInfo(activity.packageName, 0).versionName ?: ""
    return DeviceInfo(
        manufacturer = Build.MANUFACTURER,
        model = Build.MODEL,
        soc = "${Build.SOC_MANUFACTURER} ${Build.SOC_MODEL}",
        androidSdk = Build.VERSION.SDK_INT,
        totalRamBytes = memory.totalMem.toULong(),
        appVersion = version,
        startedAt = Instant.now().toString(),
    )
}

/** The bench: runs the engine's probe and micro-benchmarks, shows each result as it completes, and shares the report. */
@Composable
fun AppScreen(activity: ComponentActivity) {
    val lines = remember { mutableStateListOf<BenchLine>() }
    var phase by remember { mutableStateOf(Phase.READY) }
    var report by remember { mutableStateOf<String?>(null) }
    var reportFile by remember { mutableStateOf<String?>(null) }
    val listState = rememberLazyListState()
    LaunchedEffect(lines.size) {
        if (lines.isNotEmpty()) listState.animateScrollToItem(lines.size - 1)
    }

    /** Runs one of the engine's benches on its own thread, showing each line as it comes and keeping its report. */
    fun launch(name: String, file: File, body: (BenchHost) -> String) {
        lines.clear()
        report = null
        phase = Phase.RUNNING
        // The screen stays on so the phone runs the bench as it would run a turn in play.
        activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val main = Handler(Looper.getMainLooper())
        val power = activity.getSystemService(PowerManager::class.java)
        val host = object : BenchHost {
            override fun thermalStatus(): Int = power.currentThermalStatus
            override fun onLine(line: BenchLine) {
                main.post { lines.add(line) }
            }
        }
        thread(name = name) {
            val text = body(host)
            main.post {
                report = text
                reportFile = file.absolutePath
                phase = Phase.DONE
                activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }
    }

    fun start() {
        val info = device(activity)
        val file = File(activity.getExternalFilesDir(null), "S0.07-report.json")
        launch("phx-bench", file) { host -> runBench(info, host, file.absolutePath) }
    }

    fun startWorld() {
        val data = File(activity.filesDir, "data")
        val runDir = File(activity.filesDir, "run")
        val file = File(activity.getExternalFilesDir(null), "S0.26-world-report.json")
        launch("phx-world", file) { host ->
            data.deleteRecursively()
            unpack(activity, "data", data)
            runDir.mkdirs()
            runWorld(host, data.absolutePath, runDir.absolutePath, WORLD_TURNS, file.absolutePath)
        }
    }

    fun startLoad() {
        val load = File(activity.filesDir, "load")
        val file = File(activity.getExternalFilesDir(null), "S0.26-load-report.json")
        launch("phx-load", file) { host ->
            load.deleteRecursively()
            unpack(activity, "load", load)
            val volumes = File(load, "volumes.toml").absolutePath
            runLoad(host, volumes, activity.cacheDir.absolutePath, file.absolutePath)
        }
    }

    /** The whole programme, one report: the bench, the world's turns and the full load, one after another. */
    fun startProgramme() {
        val info = device(activity)
        val data = File(activity.filesDir, "data")
        val runDir = File(activity.filesDir, "run")
        val load = File(activity.filesDir, "load")
        val file = File(activity.getExternalFilesDir(null), "S0.26-device-report.json")
        launch("phx-programme", file) { host ->
            data.deleteRecursively()
            unpack(activity, "data", data)
            runDir.deleteRecursively()
            runDir.mkdirs()
            load.deleteRecursively()
            unpack(activity, "load", load)
            val volumes = File(load, "volumes.toml").absolutePath
            runProgramme(
                info, host, data.absolutePath, runDir.absolutePath, WORLD_TURNS, volumes,
                activity.cacheDir.absolutePath, file.absolutePath,
            )
        }
    }

    fun share() {
        val text = report ?: return
        val send = Intent(Intent.ACTION_SEND)
            .setType("application/json")
            .putExtra(Intent.EXTRA_SUBJECT, "Phoenix device report")
            .putExtra(Intent.EXTRA_TEXT, text)
        activity.startActivity(Intent.createChooser(send, "Share the report"))
    }

    Column(Modifier.fillMaxSize().safeDrawingPadding().padding(16.dp)) {
        Text("Project Phoenix bench", style = MaterialTheme.typography.titleLarge)
        Text(
            when (phase) {
                Phase.READY -> "Plug the phone in and close other apps. The bench takes a few minutes and about 4 GB; the " +
                    "world opens three countries' population, then runs $WORLD_TURNS turns; the full load builds the " +
                    "finished world's stores, about 5 GB, and runs a month."
                Phase.RUNNING -> "Running… keep the app open."
                Phase.DONE -> "Done. The report is at ${reportFile ?: "?"}"
            },
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.padding(vertical = 8.dp),
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = ::start, enabled = phase != Phase.RUNNING) { Text("Run bench") }
            Button(onClick = ::startWorld, enabled = phase != Phase.RUNNING) { Text("Run world") }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = ::startLoad, enabled = phase != Phase.RUNNING) { Text("Full load") }
            Button(onClick = ::startProgramme, enabled = phase != Phase.RUNNING) { Text("Run all") }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = ::share, enabled = report != null) { Text("Share report") }
        }
        HorizontalDivider(Modifier.padding(vertical = 8.dp))
        LazyColumn(Modifier.fillMaxWidth().weight(1f), state = listState) {
            items(lines) { line -> Line(line) }
        }
    }
}

@Composable
private fun Line(line: BenchLine) {
    val colour = when (line.verdict) {
        "met" -> Color(0xFF2E7D32)
        "missed" -> Color(0xFFC62828)
        else -> MaterialTheme.colorScheme.onSurface
    }
    Column(Modifier.padding(vertical = 4.dp)) {
        Text("${line.section} · ${line.name}", style = MaterialTheme.typography.labelMedium)
        Text(
            line.value + if (line.target.isNotEmpty()) "   (target ${line.target}: ${line.verdict})" else "",
            color = colour,
            style = MaterialTheme.typography.bodyMedium,
        )
        Text(
            "thermal: ${THERMAL.getOrElse(line.thermalStatus) { line.thermalStatus.toString() }}",
            style = MaterialTheme.typography.labelSmall,
        )
    }
}
