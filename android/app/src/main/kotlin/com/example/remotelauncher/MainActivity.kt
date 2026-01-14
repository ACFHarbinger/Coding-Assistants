package com.example.remotelauncher

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import com.example.remotelauncher.ui.ConnectionScreen
import com.example.remotelauncher.ui.ModelSelectionScreen
import com.example.remotelauncher.ui.TaskExecutionScreen
import com.example.remotelauncher.viewmodel.MainViewModel
import com.example.remotelauncher.viewmodel.Screen

class MainActivity : ComponentActivity() {
    private val viewModel: MainViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            RemoteLauncherTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    val state by viewModel.state.collectAsState()
                    
                    when (state.currentScreen) {
                        is Screen.Connection -> ConnectionScreen(
                            state = state,
                            onConnect = { viewModel.connectToServer(it) }
                        )
                        is Screen.ModelSelection -> ModelSelectionScreen(
                            state = state,
                            onUpdateRole = { index, role -> viewModel.updateRole(index, role) },
                            onAddRole = { viewModel.addRole() },
                            onRemoveRole = { viewModel.removeRole(it) },
                            onNext = { viewModel.navigateTo(Screen.TaskExecution) },
                            onBack = { viewModel.disconnect() }
                        )
                        is Screen.TaskExecution -> TaskExecutionScreen(
                            state = state,
                            onUpdateTask = { viewModel.updateTask(it) },
                            onExecuteTask = { viewModel.executeTask() },
                            onCancelTask = { viewModel.cancelTask() },
                            onBack = { viewModel.navigateTo(Screen.ModelSelection) },
                            onDisconnect = { viewModel.disconnect() }
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun RemoteLauncherTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Color(0xFFA855F7),
            secondary = Color(0xFF38BDF8),
            background = Color(0xFF0F172A),
            surface = Color(0xFF1E293B),
            surfaceVariant = Color(0xFF334155),
            onSurface = Color.White,
            onSurfaceVariant = Color.LightGray
        ),
        content = content
    )
}
