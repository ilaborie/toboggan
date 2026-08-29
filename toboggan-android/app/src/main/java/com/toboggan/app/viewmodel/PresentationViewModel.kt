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
import uniffi.toboggan.ErrorKind
import uniffi.toboggan.Slide
import uniffi.toboggan.ClientRole
import uniffi.toboggan.PresentationState
import uniffi.toboggan.TobogganClient

data class PresentationUiState(
    val presentationTitle: String = "Presentation title - Date",
    val connectionStatus: ConnectionStatus = ConnectionStatus.Closed,
    // Audience until the server says otherwise, mirroring the core default: a
    // role that has not arrived is the one that can do the least. Held as a
    // non-null value so no branch can read "unknown" as "allowed".
    val role: ClientRole = ClientRole.AUDIENCE,
    val isRegistered: Boolean = false,
    val currentSlideIndex: Int? = null,
    val previousSlideIndex: Int? = null,
    val nextSlideIndex: Int? = null,
    val totalSlides: Int = 0,
    val currentSlide: Slide? = null,
    val nextSlideTitle: String = "<End of presentation>",
    val errorMessage: String? = null,
    // A refusal is a permissions answer, not a broken connection, and belongs
    // beside the controls rather than in the error banner.
    val notice: String? = null,
    val currentStep: Int = 0,
    // Dots to draw: one per state the slide can be in, which is one more than
    // the number of reveals. Zero when the server has not counted them.
    val stepStates: Int = 0
) {
    val slideProgress: String
        get() = when (currentSlideIndex) {
            null -> "Ready to Start"
            else -> "${currentSlideIndex + 1} of $totalSlides"
        }

    val currentSlideTitle: String
        get() = currentSlide?.title ?: "Ready to Start"

    val isPresenter: Boolean
        get() = role == ClientRole.PRESENTER

    // Derived rather than stored, so a role arriving after a state change still
    // reaches the buttons. Stored, they were computed once from slide adjacency
    // alone and stayed enabled for a client the server would refuse.
    val canGoPrevious: Boolean
        get() = isPresenter && previousSlideIndex != null

    val canGoNext: Boolean
        get() = isPresenter && nextSlideIndex != null
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
                        totalSlides = talk.titles.size
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
                handleError(ErrorKind.TRANSPORT, "Could not fetch talk information from server")
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

    override fun onError(kind: ErrorKind, error: String) {
        viewModelScope.launch(Dispatchers.Main) {
            handleError(kind, error)
        }
    }

    override fun onRegistered(clientId: String, role: ClientRole) {
        // Said plainly, because the alternative is discovering it by pressing a
        // button and having the server refuse: a client that is not on the
        // presenting machine and carries no token is audience. The role gates
        // `canGoPrevious`/`canGoNext`, and the notice says why they are dead.
        _uiState.update {
            it.copy(
                role = role,
                isRegistered = true,
                notice = if (role == ClientRole.AUDIENCE) {
                    "Watching — this client cannot present."
                } else {
                    null
                }
            )
        }
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
                    stepStates = stepStates(state.stepCount)
                )
            }
            is PresentationState.Done -> {
                updatePresentationState(
                    currentSlideIndex = state.current.toInt(),
                    previousSlideIndex = state.previous?.toInt(),
                    nextSlideIndex = null,
                    currentStep = state.currentStep.toInt(),
                    stepStates = stepStates(state.stepCount)
                )
            }
        }
    }

    // How many dots the slide is worth.
    // 
    // The server counts *additional* reveals, so a slide with two of them has
    // three states. `null` means it has not counted at all, which is not the
    // same as counting zero — see `Slide.stepCount` on the Rust side.
    private fun stepStates(revealCount: UInt?): Int =
        revealCount?.let { it.toInt() + 1 } ?: 0

    private fun updatePresentationState(
        currentSlideIndex: Int?,
        previousSlideIndex: Int? = null,
        nextSlideIndex: Int? = null,
        currentStep: Int = 0,
        stepStates: Int = 0
    ) {
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
                currentSlideIndex = currentSlideIndex,
                previousSlideIndex = previousSlideIndex,
                nextSlideIndex = nextSlideIndex,
                currentSlide = currentSlide,
                nextSlideTitle = nextSlideTitle,
                currentStep = currentStep,
                stepStates = stepStates
            )
        }
    }

    private fun handleError(kind: ErrorKind, error: String) {
        _uiState.update { state ->
            when (kind) {
                // The server answered and declined. Reporting that as a
                // connection error blamed the network for a permissions
                // decision — and overwrote the status of a socket that is fine.
                ErrorKind.SERVER -> state.copy(notice = error)
                ErrorKind.TRANSPORT -> state.copy(
                    connectionStatus = ConnectionStatus.Error(error),
                    errorMessage = error
                )
            }
        }
    }

    fun clearError() {
        _uiState.update { it.copy(errorMessage = null) }
    }

    // MARK: - Actions

    fun nextStep() {
        tobogganClient?.sendCommand(Command.NextStep)
    }

    fun previousStep() {
        tobogganClient?.sendCommand(Command.PreviousStep)
    }

    fun firstSlide() {
        tobogganClient?.sendCommand(Command.First)
    }

    fun lastSlide() {
        tobogganClient?.sendCommand(Command.Last)
    }

    fun blink() {
        tobogganClient?.sendCommand(Command.Blink)
    }
}
