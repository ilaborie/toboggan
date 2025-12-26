package com.toboggan.app.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.toboggan.ClientConfig
import uniffi.toboggan.ClientNotificationHandler
import uniffi.toboggan.Command
import uniffi.toboggan.ConnectionStatus
import uniffi.toboggan.Slide
import uniffi.toboggan.State
import uniffi.toboggan.TobogganClient
import java.time.Duration

data class PresentationUiState(
    val presentationTitle: String = "Presentation title - Date",
    val connectionStatus: ConnectionStatus = ConnectionStatus.CLOSED,
    val currentSlideIndex: Int? = null,
    val totalSlides: Int = 0,
    val currentSlide: Slide? = null,
    val nextSlideTitle: String = "<End of presentation>",
    val isPlaying: Boolean = false,
    val duration: Long = 0L,
    val canGoPrevious: Boolean = false,
    val canGoNext: Boolean = false,
    val errorMessage: String? = null,
    val currentStep: Int = 0,
    val stepCount: Int = 1
) {
    val formattedDuration: String
        get() {
            val minutes = (duration / 60).toInt()
            val seconds = (duration % 60).toInt()
            return String.format("%02d:%02d", minutes, seconds)
        }

    val slideProgress: String
        get() = when (currentSlideIndex) {
            null -> "Ready to Start"
            else -> "${currentSlideIndex + 1} of $totalSlides"
        }

    val currentSlideTitle: String
        get() = currentSlide?.title ?: "Ready to Start"
}

class PresentationViewModel : ViewModel(), ClientNotificationHandler {

    private val _uiState = MutableStateFlow(PresentationUiState())
    val uiState: StateFlow<PresentationUiState> = _uiState.asStateFlow()

    private var tobogganClient: TobogganClient? = null
    private var currentState: State? = null
    private var talkLoaded = false
    private var pendingStateUpdate: State? = null

    init {
        connectToServer()
    }

    private fun connectToServer() {
        viewModelScope.launch(Dispatchers.IO) {
            val config = ClientConfig(
                // Use 10.0.2.2 for Android emulator localhost
                url = "http://10.0.2.2:8080",
                maxRetries = 3u,
                retryDelay = Duration.ofSeconds(1)
            )

            _uiState.update { it.copy(connectionStatus = ConnectionStatus.CONNECTING) }

            tobogganClient = TobogganClient(config, this@PresentationViewModel)
            tobogganClient?.connect()

            fetchTalkInfo()
        }
    }

    private fun fetchTalkInfo() {
        tobogganClient?.getTalk()?.let { talk ->
            viewModelScope.launch(Dispatchers.Main) {
                _uiState.update { state ->
                    state.copy(
                        presentationTitle = "${talk.title} - ${talk.date}",
                        totalSlides = talk.slides.size
                    )
                }
                talkLoaded = true

                // Process any pending state updates
                pendingStateUpdate?.let { pending ->
                    handleStateChange(pending)
                    pendingStateUpdate = null
                }
            }
        } ?: run {
            viewModelScope.launch(Dispatchers.Main) {
                handleError("Could not fetch talk information from server")
            }
        }
    }

    // MARK: - ClientNotificationHandler implementation

    override fun onStateChange(state: State) {
        viewModelScope.launch(Dispatchers.Main) {
            handleStateChange(state)
        }
    }

    override fun onTalkChange(state: State) {
        viewModelScope.launch(Dispatchers.IO) {
            fetchTalkInfo()
        }
        viewModelScope.launch(Dispatchers.Main) {
            handleStateChange(state)
        }
    }

    override fun onConnectionStatusChange(status: ConnectionStatus) {
        viewModelScope.launch(Dispatchers.Main) {
            _uiState.update { it.copy(connectionStatus = status) }
        }
    }

    override fun onError(error: String) {
        viewModelScope.launch(Dispatchers.Main) {
            handleError(error)
        }
    }

    // MARK: - State handling

    private fun handleStateChange(state: State) {
        currentState = state

        when (state) {
            is State.Init -> {
                _uiState.update { it.copy(totalSlides = state.totalSlides.toInt()) }
                updatePresentationState(currentSlideIndex = null)
            }
            is State.Running -> {
                updatePresentationState(
                    currentSlideIndex = state.current.toInt(),
                    isPlaying = true,
                    duration = state.totalDuration.seconds,
                    previousSlideIndex = state.previous?.toInt(),
                    nextSlideIndex = state.next?.toInt(),
                    currentStep = state.currentStep.toInt(),
                    stepCount = state.stepCount.toInt()
                )
            }
            is State.Paused -> {
                updatePresentationState(
                    currentSlideIndex = state.current.toInt(),
                    isPlaying = false,
                    duration = state.totalDuration.seconds,
                    previousSlideIndex = state.previous?.toInt(),
                    nextSlideIndex = state.next?.toInt(),
                    currentStep = state.currentStep.toInt(),
                    stepCount = state.stepCount.toInt()
                )
            }
            is State.Done -> {
                updatePresentationState(
                    currentSlideIndex = state.current.toInt(),
                    isPlaying = false,
                    duration = state.totalDuration.seconds,
                    previousSlideIndex = state.previous?.toInt(),
                    nextSlideIndex = null,
                    currentStep = state.currentStep.toInt(),
                    stepCount = state.stepCount.toInt()
                )
            }
        }
    }

    private fun updatePresentationState(
        currentSlideIndex: Int?,
        isPlaying: Boolean = false,
        duration: Long = 0L,
        previousSlideIndex: Int? = null,
        nextSlideIndex: Int? = null,
        currentStep: Int = 0,
        stepCount: Int = 1
    ) {
        val canGoPrevious = previousSlideIndex != null
        val canGoNext = nextSlideIndex != null

        // Update current slide if we have one
        val currentSlide = currentSlideIndex?.let { idx ->
            if (!talkLoaded) {
                pendingStateUpdate = currentState
                null
            } else {
                tobogganClient?.getSlide(idx.toUInt())
            }
        }

        // Fetch next slide title
        val nextSlideTitle = nextSlideIndex?.let { idx ->
            tobogganClient?.getSlide(idx.toUInt())?.title
        } ?: "<End of presentation>"

        _uiState.update { state ->
            state.copy(
                isPlaying = isPlaying,
                duration = duration,
                canGoPrevious = canGoPrevious,
                canGoNext = canGoNext,
                currentSlideIndex = currentSlideIndex,
                currentSlide = currentSlide,
                nextSlideTitle = nextSlideTitle,
                currentStep = currentStep,
                stepCount = stepCount
            )
        }
    }

    private fun handleError(error: String) {
        _uiState.update { state ->
            state.copy(
                connectionStatus = ConnectionStatus.ERROR,
                errorMessage = error
            )
        }
    }

    fun clearError() {
        _uiState.update { it.copy(errorMessage = null) }
    }

    // MARK: - Actions

    fun nextStep() {
        tobogganClient?.sendCommand(Command.NEXT_STEP)
    }

    fun previousStep() {
        tobogganClient?.sendCommand(Command.PREVIOUS_STEP)
    }

    fun firstSlide() {
        tobogganClient?.sendCommand(Command.FIRST)
    }

    fun lastSlide() {
        tobogganClient?.sendCommand(Command.LAST)
    }

    fun togglePlayPause() {
        val command = if (_uiState.value.isPlaying) Command.PAUSE else Command.RESUME
        tobogganClient?.sendCommand(command)
    }

    fun blink() {
        tobogganClient?.sendCommand(Command.BLINK)
    }
}
