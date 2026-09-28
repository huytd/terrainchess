#!/usr/bin/env python3
"""Synthesize retro chiptune sound effects for Terrain Chess.

Generates mono 16-bit 22 050 Hz WAV files into assets/sfx/.
"""

import os
import wave
import numpy as np

SAMPLE_RATE = 22050


def save_wav(filename: str, samples: np.ndarray):
    """Save 1D float array (-1.0 to 1.0) as 16-bit mono 22050 Hz WAV."""
    os.makedirs(os.path.dirname(filename), exist_ok=True)
    # Clip and convert to 16-bit PCM
    samples = np.clip(samples, -1.0, 1.0)
    int_samples = (samples * 32767).astype(np.int16)
    with wave.open(filename, "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(SAMPLE_RATE)
        wf.writeframes(int_samples.tobytes())


def square_wave(freq, t, duty=0.5):
    """Generate square/pulse wave."""
    phase = (freq * t) % 1.0
    return np.where(phase < duty, 1.0, -1.0)


def triangle_wave(freq, t):
    """Generate triangle wave."""
    phase = (freq * t) % 1.0
    return 2.0 * np.abs(2.0 * (phase - np.floor(phase + 0.5))) - 1.0


def make_select():
    """select: soft click (< 0.6s)"""
    dur = 0.04
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    # Pitch drops rapidly from 1200 Hz to 400 Hz
    freq = 1200.0 - 800.0 * (t / dur)
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    wave = np.sin(phase) * 0.7 + triangle_wave(freq, t) * 0.3
    # Sharp exponential decay
    env = np.exp(-t * 90.0)
    return wave * env * 0.8


def make_move():
    """move: wooden thud (< 0.6s)"""
    dur = 0.12
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    # Low frequency drop 160Hz -> 45Hz
    freq = 160.0 * np.exp(-t * 18.0) + 40.0
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    # Triangle wave + sine for hollow woody character
    body = triangle_wave(freq, t) * 0.6 + np.sin(phase) * 0.4
    # Initial subtle tap transient
    noise = np.random.uniform(-1, 1, len(t)) * np.exp(-t * 120.0) * 0.3
    env = np.exp(-t * 28.0)
    return (body + noise) * env * 0.9


def make_capture():
    """capture: noise burst + low hit (< 0.6s)"""
    dur = 0.22
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    # Noise burst for impact crack
    noise = np.random.uniform(-1, 1, len(t))
    noise_env = np.exp(-t * 35.0)
    noise_part = noise * noise_env * 0.7

    # Low punchy hit
    freq = 140.0 * np.exp(-t * 22.0) + 30.0
    hit = square_wave(freq, t, duty=0.3) * 0.5 + triangle_wave(freq, t) * 0.5
    hit_env = np.exp(-t * 16.0)
    hit_part = hit * hit_env * 0.8

    return (noise_part + hit_part) * 0.85


def make_splash():
    """splash: filtered noise (< 0.6s)"""
    dur = 0.35
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    np.random.seed(42)
    raw_noise = np.random.uniform(-1, 1, len(t))

    # Resonant lowpass/bandpass filtering via 2-pole IIR filter
    # Simulating liquid slosh/splash
    filtered = np.zeros_like(raw_noise)
    # Center frequency glides from 1200 Hz down to 350 Hz
    f_center = 1200.0 * np.exp(-t * 6.0) + 300.0
    q = 2.0
    for i in range(2, len(t)):
        w0 = 2 * np.pi * f_center[i] / SAMPLE_RATE
        alpha = np.sin(w0) / (2 * q)
        b0 = alpha
        b2 = -alpha
        a0 = 1 + alpha
        a1 = -2 * np.cos(w0)
        a2 = 1 - alpha
        filtered[i] = (b0 * raw_noise[i] + b2 * raw_noise[i - 2] - a1 * filtered[i - 1] - a2 * filtered[i - 2]) / a0

    env = (1.0 - np.exp(-t * 80.0)) * np.exp(-t * 9.0)
    # Add subtle bubble drops
    bubble_phase = 2 * np.pi * (500 + 300 * np.sin(2 * np.pi * 12 * t)) * t
    bubble = np.sin(bubble_phase) * np.exp(-t * 12.0) * 0.25

    sig = filtered * 2.2 + bubble
    return sig * env * 0.8


def make_spell():
    """spell: rising shimmer (< 0.6s)"""
    dur = 0.45
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    # Rapid rising arpeggio / frequency sweep from 350 Hz to 1400 Hz
    freq = 350.0 + 1050.0 * (t / dur) ** 1.5
    # Shimmer modulation (vibrato / tremolo)
    vibrato = 1.0 + 0.05 * np.sin(2 * np.pi * 28.0 * t)
    phase = 2 * np.pi * np.cumsum(freq * vibrato) / SAMPLE_RATE

    sig = (
        square_wave(freq * 0.5, t, duty=0.25) * 0.2
        + triangle_wave(freq, t) * 0.5
        + np.sin(phase * 2.0) * 0.3
    )
    env = np.sin(np.pi * np.clip(t / dur, 0, 1) ** 0.6) * np.exp(-t * 1.5)
    return sig * env * 0.75


def make_pickup():
    """pickup: two quick bright notes (< 0.6s)"""
    dur = 0.20
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    n1_dur = 0.08
    # Note 1: E6 (1318.5 Hz), Note 2: B6 (1975.5 Hz)
    sig = np.zeros_like(t)
    mask1 = t < n1_dur
    t1 = t[mask1]
    sig[mask1] = (square_wave(1318.5, t1, duty=0.5) * 0.6 + triangle_wave(1318.5, t1) * 0.4) * np.exp(-t1 * 18.0)

    mask2 = t >= n1_dur
    t2 = t[mask2] - n1_dur
    sig[mask2] = (square_wave(1975.5, t2, duty=0.5) * 0.6 + triangle_wave(1975.5, t2) * 0.4) * np.exp(-t2 * 14.0)

    return sig * 0.75


def make_check():
    """check: tense two-note (< 0.6s)"""
    dur = 0.32
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    n1_dur = 0.12
    # Minor 2nd / tritone tension: F#5 (740 Hz) -> G5 (784 Hz) or C5 (523 Hz) -> F#4 (370 Hz)
    sig = np.zeros_like(t)
    mask1 = t < n1_dur
    t1 = t[mask1]
    sig[mask1] = square_wave(587.33, t1, duty=0.25) * 0.7 * np.exp(-t1 * 10.0)  # D5

    mask2 = t >= n1_dur
    t2 = t[mask2] - n1_dur
    # Low tense dissonant note G#4 (415.3 Hz) with subtle vibrato
    freq2 = 415.3 + 10.0 * np.sin(2 * np.pi * 14.0 * t2)
    sig[mask2] = square_wave(freq2, t2, duty=0.2) * 0.8 * np.exp(-t2 * 7.0)

    return sig * 0.75


def make_win():
    """win: short major jingle, about 1.5 s"""
    dur = 1.45
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    # Fanfare: C5 (523), E5 (659), G5 (784), C6 (1046)
    notes = [
        (0.00, 0.18, 523.25),
        (0.18, 0.18, 659.25),
        (0.36, 0.18, 783.99),
        (0.54, 0.91, 1046.50),
    ]
    sig = np.zeros_like(t)
    for start, note_dur, f in notes:
        mask = (t >= start) & (t < start + note_dur)
        tn = t[mask] - start
        wave = square_wave(f, tn, duty=0.5) * 0.4 + triangle_wave(f, tn) * 0.4 + np.sin(2 * np.pi * f * tn) * 0.2
        # Harmonize final note with major third (E6 = 1318.5) and fifth (G6 = 1568)
        if f > 1000.0:
            wave += triangle_wave(1318.5, tn) * 0.25 + triangle_wave(1567.98, tn) * 0.2
            env = np.exp(-tn * 2.2)
        else:
            env = np.exp(-tn * 4.0)
        sig[mask] += wave * env

    return sig * 0.7


def make_lose():
    """lose: short falling minor jingle, about 1.5 s"""
    dur = 1.45
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    # Sad falling notes: Eb4 (311), D4 (293.7), Db4 (277.2), C4 (261.6)
    notes = [
        (0.00, 0.24, 392.0),   # G4
        (0.24, 0.24, 311.13),  # Eb4
        (0.48, 0.24, 293.66),  # D4
        (0.72, 0.73, 261.63),  # C4
    ]
    sig = np.zeros_like(t)
    for start, note_dur, f in notes:
        mask = (t >= start) & (t < start + note_dur)
        tn = t[mask] - start
        # Add downward pitch slide and sad vibrato
        slide = f * (1.0 - 0.04 * (tn / note_dur))
        vibrato = 1.0 + 0.02 * np.sin(2 * np.pi * 5.0 * tn)
        wave = triangle_wave(slide * vibrato, tn) * 0.6 + square_wave(slide * vibrato, tn, duty=0.25) * 0.4
        if f < 270.0:
            # Low minor drone
            wave += triangle_wave(196.0, tn) * 0.3  # G3
            env = np.exp(-tn * 2.0)
        else:
            env = np.exp(-tn * 3.5)
        sig[mask] += wave * env

    return sig * 0.75


def make_card():
    """card: paper flick (< 0.6s)"""
    dur = 0.09
    t = np.linspace(0, dur, int(SAMPLE_RATE * dur), endpoint=False)
    # Quick noise flick + high-frequency friction sweep
    np.random.seed(123)
    noise = np.random.uniform(-1, 1, len(t))
    freq = 2400.0 * np.exp(-t * 40.0) + 400.0
    flick = triangle_wave(freq, t) * 0.4 + noise * 0.6
    env = (1.0 - np.exp(-t * 200.0)) * np.exp(-t * 45.0)
    return flick * env * 0.85


def main():
    sounds = {
        "select": make_select(),
        "move": make_move(),
        "capture": make_capture(),
        "splash": make_splash(),
        "spell": make_spell(),
        "pickup": make_pickup(),
        "check": make_check(),
        "win": make_win(),
        "lose": make_lose(),
        "card": make_card(),
    }

    out_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "assets", "sfx")
    for name, samples in sounds.items():
        path = os.path.join(out_dir, f"{name}.wav")
        save_wav(path, samples)
        dur = len(samples) / SAMPLE_RATE
        print(f"Generated {name}.wav: {dur:.3f}s ({len(samples)} samples)")


if __name__ == "__main__":
    main()
