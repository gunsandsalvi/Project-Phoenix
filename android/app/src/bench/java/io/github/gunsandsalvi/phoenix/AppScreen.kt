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

private enum class Phase { READY, RUNNING, DONE }

private val THERMAL = listOf("none", "light", "moderate", "severe", "critical", "emergency", "shutdown")

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

    fun start() {
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
        val info = device(activity)
        val file = File(activity.getExternalFilesDir(null), "S0.07-report.json")
        thread(name = "phx-bench") {
            val text = runBench(info, host, file.absolutePath)
            main.post {
                report = text
                reportFile = file.absolutePath
                phase = Phase.DONE
                activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }
    }

    fun share() {
        val text = report ?: return
        val send = Intent(Intent.ACTION_SEND)
            .setType("application/json")
            .putExtra(Intent.EXTRA_SUBJECT, "Phoenix device report S0.07")
            .putExtra(Intent.EXTRA_TEXT, text)
        activity.startActivity(Intent.createChooser(send, "Share the report"))
    }

    Column(Modifier.fillMaxSize().safeDrawingPadding().padding(16.dp)) {
        Text("Project Phoenix bench", style = MaterialTheme.typography.titleLarge)
        Text(
            when (phase) {
                Phase.READY -> "Plug the phone in, close other apps, and run. It takes a few minutes and about 4 GB."
                Phase.RUNNING -> "Running… keep the app open."
                Phase.DONE -> "Done. The report is at ${reportFile ?: "?"}"
            },
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.padding(vertical = 8.dp),
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = ::start, enabled = phase != Phase.RUNNING) { Text("Run") }
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
