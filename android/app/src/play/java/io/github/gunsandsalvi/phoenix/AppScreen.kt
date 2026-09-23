package io.github.gunsandsalvi.phoenix

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** The game's screen, which waits for a world to show. */
@Composable
fun AppScreen(@Suppress("UNUSED_PARAMETER") activity: ComponentActivity) {
    Column(Modifier.fillMaxSize().safeDrawingPadding().padding(16.dp)) {
        Text("Project Phoenix", style = MaterialTheme.typography.titleLarge)
        Text("The world is not built yet.")
    }
}
