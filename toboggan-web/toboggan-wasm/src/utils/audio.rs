use wasm_bindgen::UnwrapThrowExt;
use web_sys::{AudioContext, GainNode, OscillatorNode, OscillatorType};

const G5: f32 = 784.0;
const C5: f32 = 523.25;
const C4: f32 = 261.63;
const E4: f32 = 329.63;
const G4: f32 = 392.0;

fn create_oscillator(
    context: &AudioContext,
    frequency: f32,
    start_time: f64,
    duration: f64,
    volume: f32,
) -> Option<(OscillatorNode, GainNode)> {
    let oscillator = context.create_oscillator().ok()?;
    let gain = context.create_gain().ok()?;

    oscillator.connect_with_audio_node(&gain).ok();
    oscillator.set_type(OscillatorType::Sine);

    oscillator
        .frequency()
        .set_value_at_time(frequency, start_time)
        .ok()?;

    let gain_param = gain.gain();
    gain_param.set_value_at_time(0.0, start_time).ok()?;
    gain_param
        .linear_ramp_to_value_at_time(volume, start_time + 0.02)
        .ok()?;
    gain_param
        .exponential_ramp_to_value_at_time(0.01, start_time + duration)
        .ok()?;

    oscillator.start_with_when(start_time).ok()?;
    oscillator.stop_with_when(start_time + duration).ok()?;

    Some((oscillator, gain))
}

pub fn play_chime() {
    let Ok(context) = AudioContext::new() else {
        return;
    };
    let Ok(oscillator) = context.create_oscillator() else {
        return;
    };
    let Ok(gain) = context.create_gain() else {
        return;
    };

    oscillator.connect_with_audio_node(&gain).unwrap_throw();
    gain.connect_with_audio_node(&context.destination())
        .unwrap_throw();

    let now = context.current_time();

    oscillator
        .frequency()
        .set_value_at_time(G5, now)
        .unwrap_throw();
    oscillator
        .frequency()
        .set_value_at_time(C5, now + 0.1)
        .unwrap_throw();

    gain.gain().set_value_at_time(0.3, now).unwrap_throw();
    gain.gain()
        .exponential_ramp_to_value_at_time(0.01, now + 0.5)
        .unwrap_throw();

    oscillator.start_with_when(now).unwrap_throw();
    oscillator.stop_with_when(now + 0.5).unwrap_throw();
}

pub fn play_tada() {
    let Ok(context) = AudioContext::new() else {
        return;
    };
    let Ok(master_gain) = context.create_gain() else {
        return;
    };

    master_gain
        .connect_with_audio_node(&context.destination())
        .unwrap_throw();
    let now = context.current_time();
    master_gain
        .gain()
        .set_value_at_time(0.3, now)
        .unwrap_throw();

    let notes = [
        (C4, 0.0, 0.15),
        (E4, 0.1, 0.15),
        (G4, 0.2, 0.2),
        (C5, 0.3, 0.4),
    ];

    for (freq, start, duration) in notes {
        if let Some((_, gain)) = create_oscillator(&context, freq, now + start, duration, 0.4) {
            gain.connect_with_audio_node(&master_gain).unwrap_throw();
        }
    }

    if let Some((harmonic, harmonic_gain)) = create_oscillator(&context, G5, now + 0.3, 0.6, 0.2) {
        harmonic.set_type(OscillatorType::Triangle);
        harmonic_gain
            .connect_with_audio_node(&master_gain)
            .unwrap_throw();
    }
}
