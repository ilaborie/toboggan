package com.toboggan.app.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.Duration
import uniffi.toboggan.ClientConfig
import uniffi.toboggan.ClientNotificationHandler
import uniffi.toboggan.Command
import uniffi.toboggan.ConnectionStatus
import uniffi.toboggan.Slide
import uniffi.toboggan.ClientRole
import uniffi.toboggan.PresentationState
import uniffi.toboggan.TobogganClient

data class PresentationUiState(
    val presentationTitle: String = "Presentation title - Date",
    val connectionStatus: ConnectionStatus = ConnectionStatus.Closed,
    val role: ClientRole? = null,
    val currentSlideIndex: Int? = null,
    val totalSlides: Int = 0,
    val currentSlide: Slide? = null,
    val nextSlideTitle: String = "<End of presentation>",
    val canGoPrevious: Boolean = false,
    val canGoNext: Boolean = false,
    val errorMessage: String? = null,
    val currentStep: Int = 0,
    val stepCount: Int = 1
) {
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
    private var currentState: PresentationState? = null
    private var talkLoaded = false
    private var pendingStateUpdate: PresentationState? = null

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

            _uiState.update { it.copy(connectionStatus = ConnectionStatus.Connecting) }

            tobogganClient = TobogganClient(config, "Android Remote", this@PresentationViewModel)
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

    override fun onStateChange(state: PresentationState) {
        viewModelScope.launch(Dispatchers.Main) {
            handleStateChange(state)
        }
    }

    override fun onTalkChange(state: PresentationState) {
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

    override fun onRegistered(clientId: String, role: ClientRole) {
        // Said plainly, because the alternative is discovering it by pressing a
        // button and having the server refuse: a client that is not on the
        // presenting machine and carries no token is audience.
        _uiState.update { it.copy(role = role) }
    }

    override fun onClientConnected(clientId: String, name: String) {
        // Another client connected - no UI action needed
    }

    override fun onClientDisconnected(clientId: String, name: String) {
        // Another client disconnected - no UI action needed
    }

    // MARK: - State handling

    private fun handleStateChange(state: PresentationState) {
        currentState = state

        when (state) {
            is PresentationState.Init -> {
                _uiState.update { it.copy(totalSlides = state.totalSlides.toInt()) }
                updatePresentationState(currentSlideIndex = null)
            }
            is PresentationState.Running -> {
                updatePresentationState(
                    currentSlideIndex = state.current.toInt(),
                    previousSlideIndex = state.previous?.toInt(),
                    nextSlideIndex = state.next?.toInt(),
                    currentStep = state.currentStep.toInt(),
                    stepCount = state.stepCount.toInt()
                )
            }
            is PresentationState.Done -> {
                updatePresentationState(
                    currentSlideIndex = state.current.toInt(),
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
                connectionStatus = ConnectionStatus.Error(error),
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

    fun blink() {
        tobogganClient?.sendCommand(Command.BLINK)
    }
}
