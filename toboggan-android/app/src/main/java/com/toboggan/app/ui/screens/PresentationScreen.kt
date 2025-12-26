package com.toboggan.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.toboggan.app.ui.components.CurrentSlideCard
import com.toboggan.app.ui.components.NavigationControls
import com.toboggan.app.ui.components.TopBar
import com.toboggan.app.viewmodel.PresentationViewModel

@Composable
fun PresentationScreen(
    viewModel: PresentationViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()

    Scaffold { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Top bar with title, connection status, and first/last buttons
            TopBar(
                uiState = uiState,
                onFirstSlide = viewModel::firstSlide,
                onLastSlide = viewModel::lastSlide
            )

            // Current slide display (takes remaining space)
            CurrentSlideCard(
                uiState = uiState,
                modifier = Modifier.weight(1f)
            )

            // Navigation controls at the bottom
            NavigationControls(
                uiState = uiState,
                onBlink = viewModel::blink,
                onPrevious = viewModel::previousStep,
                onNext = viewModel::nextStep
            )
        }

        // Error dialog
        uiState.errorMessage?.let { errorMessage ->
            AlertDialog(
                onDismissRequest = viewModel::clearError,
                title = { Text("Error") },
                text = { Text(errorMessage) },
                confirmButton = {
                    TextButton(onClick = viewModel::clearError) {
                        Text("OK")
                    }
                }
            )
        }
    }
}
