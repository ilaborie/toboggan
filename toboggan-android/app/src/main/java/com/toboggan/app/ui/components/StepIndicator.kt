package com.toboggan.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp

@Composable
fun StepIndicator(
    currentStep: Int,
    stepCount: Int,
    modifier: Modifier = Modifier
) {
    if (stepCount <= 1) return

    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        repeat(stepCount) { index ->
            StepDot(
                state = when {
                    index < currentStep -> StepState.Done
                    index == currentStep -> StepState.Current
                    else -> StepState.Remaining
                }
            )
        }
    }
}

private enum class StepState {
    Done,
    Current,
    Remaining
}

@Composable
private fun StepDot(
    state: StepState,
    modifier: Modifier = Modifier
) {
    val textColor = MaterialTheme.colorScheme.onSurface
    val accentColor = MaterialTheme.colorScheme.tertiary
    val dotSize = 12.dp
    val borderWidth = 2.dp

    Box(
        modifier = modifier
            .size(dotSize)
            .clip(CircleShape)
            .then(
                when (state) {
                    StepState.Done -> Modifier.background(textColor)
                    StepState.Current -> Modifier.background(accentColor)
                    StepState.Remaining -> Modifier.border(borderWidth, textColor, CircleShape)
                }
            )
    )
}
