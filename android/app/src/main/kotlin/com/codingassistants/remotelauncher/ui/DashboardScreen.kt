package com.codingassistants.remotelauncher.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.codingassistants.remotelauncher.network.WakeRecord
import com.codingassistants.remotelauncher.viewmodel.AppState

@Composable
fun DashboardScreen(
    state: AppState,
    onResolveWake: (String, Boolean) -> Unit,
    onRefreshWakes: () -> Unit,
    onConfigureTask: () -> Unit,
    onDisconnect: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(16.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "Dashboard",
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.primary,
            )
            Button(
                onClick = onDisconnect,
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
            ) {
                Text("Disconnect")
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Status: Connected to ${state.serverAddress}",
                    color = MaterialTheme.colorScheme.secondary,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Button(
                    onClick = onConfigureTask,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text("Configure & Start Task")
                }
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "Pending Human Approvals",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            TextButton(onClick = onRefreshWakes) {
                Text("Refresh")
            }
        }

        Spacer(modifier = Modifier.height(8.dp))

        if (state.pendingWakes.isEmpty()) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(100.dp)
                        .background(Color(0x1AFFFFFF), shape = MaterialTheme.shapes.medium),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "No pending approvals.",
                    color = Color.LightGray,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(state.pendingWakes) { wake ->
                    WakeCard(wake = wake, onResolve = onResolveWake)
                }
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        Text(
            text = "Active Events Log",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
        )

        Spacer(modifier = Modifier.height(8.dp))

        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .weight(1f)
                    .background(Color(0xFF0F172A), shape = MaterialTheme.shapes.medium)
                    .padding(12.dp),
        ) {
            LazyColumn {
                items(state.activeEvents.reversed()) { event ->
                    Text(
                        text = "[${event.source}] ${event.event_type}: ${event.content.take(
                            100,
                        )}${if (event.content.length > 100) "..." else ""}",
                        color = Color.Green,
                        fontSize = 12.sp,
                        modifier = Modifier.padding(bottom = 4.dp),
                    )
                }
            }
        }
    }
}

@Composable
fun WakeCard(
    wake: WakeRecord,
    onResolve: (String, Boolean) -> Unit,
) {
    val context = wake.toDisplayContext()
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.Top,
            ) {
                Text(
                    text = context.actionLabel,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.secondary,
                    modifier = Modifier.weight(1f),
                )
                Text(
                    text = wake.id.take(8),
                    fontSize = 12.sp,
                    color = Color.Gray,
                )
            }

            if (context.requiresHumanGate) {
                Spacer(modifier = Modifier.height(8.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Default.Shield,
                        contentDescription = "Human gate required",
                        tint = Color(0xFFEAB308),
                        modifier = Modifier.padding(end = 6.dp),
                    )
                    Text(
                        text = "Human gate required",
                        color = Color(0xFFEAB308),
                        fontSize = 12.sp,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }

            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = "Target: ${context.targetLabel}",
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
            )
            context.scopeLabel?.let { scope ->
                Spacer(modifier = Modifier.height(4.dp))
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = RoundedCornerShape(12.dp),
                ) {
                    Text(
                        text = scope,
                        fontSize = 12.sp,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                    )
                }
            }
            context.messageRef?.let { ref ->
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = ref,
                    fontSize = 12.sp,
                    color = Color.Gray,
                )
            }

            Spacer(modifier = Modifier.height(8.dp))
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Color(0x1AFFFFFF),
                shape = MaterialTheme.shapes.small,
            ) {
                Text(
                    text = context.preview,
                    fontSize = 14.sp,
                    modifier = Modifier.padding(10.dp),
                )
            }

            if (context.createdAt.isNotBlank()) {
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = context.createdAt,
                    fontSize = 11.sp,
                    color = Color.Gray,
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Button(
                    onClick = { onResolve(wake.id, false) },
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                    modifier = Modifier.padding(end = 8.dp),
                ) {
                    Icon(Icons.Default.Close, contentDescription = "Reject")
                    Spacer(Modifier.width(4.dp))
                    Text("Reject")
                }

                Button(
                    onClick = { onResolve(wake.id, true) },
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF10B981)),
                ) {
                    Icon(Icons.Default.Check, contentDescription = "Approve")
                    Spacer(Modifier.width(4.dp))
                    Text("Approve")
                }
            }
        }
    }
}
