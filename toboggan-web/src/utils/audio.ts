/**
 * Audio Utilities
 * Simple audio playback functions
 */

/**
 * Play a notification chime sound
 * Uses Web Audio API to generate a simple chime
 */
export function playChime(): void {
  try {
    // Create audio context
    const AudioContextConstructor =
      window.AudioContext ||
      (window as typeof window & { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    const audioContext = new AudioContextConstructor();

    // Create oscillator for chime sound
    const oscillator = audioContext.createOscillator();
    const gainNode = audioContext.createGain();

    // Connect nodes
    oscillator.connect(gainNode);
    gainNode.connect(audioContext.destination);

    // Configure chime sound (pleasant bell-like tone)
    oscillator.frequency.setValueAtTime(800, audioContext.currentTime); // High C note
    oscillator.frequency.setValueAtTime(600, audioContext.currentTime + 0.1); // Descend

    // Configure volume envelope (fade out)
    gainNode.gain.setValueAtTime(0.3, audioContext.currentTime);
    gainNode.gain.exponentialRampToValueAtTime(0.01, audioContext.currentTime + 0.5);

    // Play for 0.5 seconds
    oscillator.start(audioContext.currentTime);
    oscillator.stop(audioContext.currentTime + 0.5);
  } catch (error) {
    // Silently fail if audio is not available or blocked
    console.debug("Could not play chime:", error);
  }
}
