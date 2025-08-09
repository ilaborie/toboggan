/**
 * Audio Utilities
 * Simple audio playback functions
 */

/**
 * Musical note frequencies in Hz
 */
const NOTES = {
  C4: 261.63,
  E4: 329.63,
  G4: 392.0,
  C5: 523.25,
  E5: 659.25,
  G5: 784.0,
  C6: 1046.5,
} as const;

/**
 * Get AudioContext constructor (with webkit fallback)
 */
function getAudioContext(): typeof AudioContext {
  return (
    window.AudioContext ||
    (window as typeof window & { webkitAudioContext: typeof AudioContext }).webkitAudioContext
  );
}

/**
 * Play a notification chime sound
 * Uses Web Audio API to generate a simple chime
 */
export function playChime(): void {
  try {
    // Create audio context
    const AudioContextConstructor = getAudioContext();
    const audioContext = new AudioContextConstructor();

    // Create oscillator for chime sound
    const oscillator = audioContext.createOscillator();
    const gainNode = audioContext.createGain();

    // Connect nodes
    oscillator.connect(gainNode);
    gainNode.connect(audioContext.destination);

    // Configure chime sound (pleasant bell-like tone)
    oscillator.frequency.setValueAtTime(NOTES.G5, audioContext.currentTime);
    oscillator.frequency.setValueAtTime(NOTES.C5, audioContext.currentTime + 0.1);

    // Configure volume envelope (fade out)
    gainNode.gain.setValueAtTime(0.3, audioContext.currentTime);
    gainNode.gain.exponentialRampToValueAtTime(0.01, audioContext.currentTime + 0.5);

    // Play for 0.5 seconds
    oscillator.start(audioContext.currentTime);
    oscillator.stop(audioContext.currentTime + 0.5);
  } catch (_error) {
    // Silently fail if audio is not available or blocked
    // console.debug("Could not play chime:", error);
  }
}

/**
 * Play a tada/celebration sound effect
 * Uses Web Audio API to generate a triumphant fanfare-like sound
 */
export function playTada(): void {
  try {
    // Create audio context
    const AudioContextConstructor = getAudioContext();
    const audioContext = new AudioContextConstructor();

    // Create multiple oscillators for rich tada sound (lower octave)
    const notes = [
      { freq: NOTES.C4, start: 0, duration: 0.1 },
      { freq: NOTES.E4, start: 0.05, duration: 0.1 },
      { freq: NOTES.G4, start: 0.1, duration: 0.15 },
      { freq: NOTES.C4, start: 0.2, duration: 0.8 }, // finale - extended
    ];

    // Create master gain for overall volume control
    const masterGain = audioContext.createGain();
    masterGain.connect(audioContext.destination);
    masterGain.gain.setValueAtTime(0.4, audioContext.currentTime);

    // Create oscillators for each note
    for (const note of notes) {
      const oscillator = audioContext.createOscillator();
      const gainNode = audioContext.createGain();

      // Connect nodes
      oscillator.connect(gainNode);
      gainNode.connect(masterGain);

      // Use sine wave for cleaner sound
      oscillator.type = "sine";

      // Schedule frequency
      oscillator.frequency.setValueAtTime(note.freq, audioContext.currentTime + note.start);

      // Create envelope for each note
      const noteStartTime = audioContext.currentTime + note.start;
      gainNode.gain.setValueAtTime(0, noteStartTime);
      gainNode.gain.linearRampToValueAtTime(0.5, noteStartTime + 0.02); // Quick attack
      gainNode.gain.exponentialRampToValueAtTime(0.01, noteStartTime + note.duration); // Decay

      // Schedule playback
      oscillator.start(noteStartTime);
      oscillator.stop(noteStartTime + note.duration);
    }

    // Add some harmonics for richness on the final note (lower harmonic)
    const harmonic = audioContext.createOscillator();
    const harmonicGain = audioContext.createGain();
    harmonic.connect(harmonicGain);
    harmonicGain.connect(masterGain);

    harmonic.type = "triangle";
    harmonic.frequency.setValueAtTime(NOTES.G5, audioContext.currentTime + 0.2); // G5 harmonic

    harmonicGain.gain.setValueAtTime(0, audioContext.currentTime + 0.2);
    harmonicGain.gain.linearRampToValueAtTime(0.15, audioContext.currentTime + 0.22);
    harmonicGain.gain.exponentialRampToValueAtTime(0.01, audioContext.currentTime + 1.0);

    harmonic.start(audioContext.currentTime + 0.2);
    harmonic.stop(audioContext.currentTime + 1.0);
  } catch (_error) {
    // Silently fail if audio is not available or blocked
    // console.debug("Could not play tada:", error);
  }
}
