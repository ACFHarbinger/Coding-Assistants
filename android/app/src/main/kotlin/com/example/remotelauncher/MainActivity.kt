package com.example.remotelauncher

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

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
                    MainScreen(viewModel)
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainScreen(viewModel: MainViewModel) {
    val scrollState = rememberLazyListState()
    
    // Auto-scroll logic
    LaunchedEffect(viewModel.logs.size) {
        if (viewModel.logs.isNotEmpty()) {
            scrollState.animateScrollToItem(viewModel.logs.size - 1)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Remote Launcher", fontWeight = FontWeight.Bold) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant
                )
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .padding(16.dp)
                .fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Connection Card
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("Connection", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                    Spacer(modifier = Modifier.height(8.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        OutlinedTextField(
                            value = viewModel.ip,
                            onValueChange = { viewModel.ip = it },
                            label = { Text("PC IP Address") },
                            modifier = Modifier.weight(1fr),
                            singleLine = true,
                            enabled = !viewModel.isConnected
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Button(
                            onClick = { if (viewModel.isConnected) viewModel.disconnect() else viewModel.connect() },
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (viewModel.isConnected) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary
                            )
                        ) {
                            Icon(if (viewModel.isConnected) Icons.Default.LinkOff else Icons.Default.Link, contentDescription = null)
                            Spacer(modifier = Modifier.width(4.dp))
                            Text(if (viewModel.isConnected) "Disconnect" else "Connect")
                        }
                    }
                    Text(
                        text = viewModel.status,
                        style = MaterialTheme.typography.bodySmall,
                        color = if (viewModel.status.contains("Error")) Color.Red else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = 8.dp)
                    )
                }
            }

            if (viewModel.isConnected) {
                // Task Configuration
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("Task Details", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                        Spacer(modifier = Modifier.height(8.dp))
                        
                        // Provider Select
                        var providerExpanded by remember { mutableStateOf(false) }
                        ExposedDropdownMenuBox(
                            expanded = providerExpanded,
                            onExpandedChange = { providerExpanded = it }
                        ) {
                            OutlinedTextField(
                                value = viewModel.selectedProvider,
                                onValueChange = {},
                                readOnly = true,
                                label = { Text("Provider") },
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = providerExpanded) },
                                modifier = Modifier.menuAnchor().fillMaxWidth()
                            )
                            ExposedDropdownMenu(
                                expanded = providerExpanded,
                                onDismissRequest = { providerExpanded = false }
                            ) {
                                viewModel.models.keys.forEach { provider ->
                                    DropdownMenuItem(
                                        text = { Text(provider) },
                                        onClick = {
                                            viewModel.selectedProvider = provider
                                            viewModel.selectedModel = viewModel.models[provider]?.firstOrNull() ?: ""
                                            providerExpanded = false
                                        }
                                    )
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(8.dp))

                        // Model Select
                        var modelExpanded by remember { mutableStateOf(false) }
                        ExposedDropdownMenuBox(
                            expanded = modelExpanded,
                            onExpandedChange = { modelExpanded = it }
                        ) {
                            OutlinedTextField(
                                value = viewModel.selectedModel,
                                onValueChange = {},
                                readOnly = true,
                                label = { Text("Model") },
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = modelExpanded) },
                                modifier = Modifier.menuAnchor().fillMaxWidth()
                            )
                            ExposedDropdownMenu(
                                expanded = modelExpanded,
                                onDismissRequest = { modelExpanded = false }
                            ) {
                                viewModel.models[viewModel.selectedProvider]?.forEach { model ->
                                    DropdownMenuItem(
                                        text = { Text(model) },
                                        onClick = {
                                            viewModel.selectedModel = model
                                            modelExpanded = false
                                        }
                                    )
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(16.dp))

                        OutlinedTextField(
                            value = viewModel.taskText,
                            onValueChange = { viewModel.taskText = it },
                            label = { Text("Task Description") },
                            modifier = Modifier.fillMaxWidth(),
                            minLines = 3
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                            if (viewModel.isRunning) {
                                Button(
                                    onClick = { viewModel.cancelTask() },
                                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                                ) {
                                    Icon(Icons.Default.Stop, contentDescription = null)
                                    Text("Cancel Task")
                                }
                            } else {
                                Button(
                                    onClick = { viewModel.launchTask() },
                                    enabled = viewModel.taskText.isNotBlank()
                                ) {
                                    Icon(Icons.Default.PlayArrow, contentDescription = null)
                                    Text("Launch Sequence")
                                }
                            }
                        }
                    }
                }

                // Logs Card
                Card(
                    modifier = Modifier.weight(1fr).fillMaxWidth(),
                    colors = CardDefaults.cardColors(containerColor = Color.Black)
                ) {
                    Column(modifier = Modifier.padding(8.dp)) {
                        Text("Agent Activity Logs", color = Color.Gray, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        Divider(modifier = Modifier.padding(vertical = 4.dp), color = Color.DarkGray)
                        LazyColumn(
                            state = scrollState,
                            modifier = Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(4.dp)
                        ) {
                            items(viewModel.logs) { log ->
                                Text(
                                    text = log,
                                    color = Color.Green,
                                    fontSize = 12.sp,
                                    fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace
                                )
                            }
                        }
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
            surfaceVariant = Color(0xFF334155)
        ),
        content = content
    )
}
