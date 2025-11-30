package com.toboggan.app.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Circle
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.toboggan.app.ui.theme.StatusClosed
import com.toboggan.app.ui.theme.StatusConnected
import com.toboggan.app.ui.theme.StatusConnecting
import com.toboggan.app.ui.theme.StatusError
import uniffi.toboggan.ConnectionStatus

@Composable
fun ConnectionStatusIndicator(
    status: ConnectionStatus,
    modifier: Modifier = Modifier
) {
    val statusColor by animateColorAsState(
        targetValue = status.toColor(),
        label = "status_color"
    )

    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = modifier
    ) {
        Icon(
            imageVector = Icons.Default.Circle,
            contentDescription = null,
            tint = statusColor,
            modifier = Modifier.size(8.dp)
        )
        Spacer(modifier = Modifier.width(6.dp))
        Text(
            text = status.displayText,
            style = MaterialTheme.typography.labelMedium,
            color = statusColor
        )
    }
}

val ConnectionStatus.displayText: String
    get() = when (this) {
        ConnectionStatus.CONNECTING -> "Connecting..."
        ConnectionStatus.CONNECTED -> "Connected"
        ConnectionStatus.CLOSED -> "Disconnected"
        ConnectionStatus.RECONNECTING -> "Reconnecting..."
        ConnectionStatus.ERROR -> "Connection Error"
    }

fun ConnectionStatus.toColor(): Color = when (this) {
    ConnectionStatus.CONNECTED -> StatusConnected
    ConnectionStatus.CONNECTING, ConnectionStatus.RECONNECTING -> StatusConnecting
    ConnectionStatus.CLOSED -> StatusClosed
    ConnectionStatus.ERROR -> StatusError
}
