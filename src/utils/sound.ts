let context: AudioContext | null = null;

function tone(frequencies: number[], duration = 0.09, volume = 0.08) {
  try {
    context ??= new AudioContext();
    const ctx = context;
    let start = ctx.currentTime;
    for (const frequency of frequencies) {
      const oscillator = ctx.createOscillator();
      const gain = ctx.createGain();
      oscillator.type = "sine";
      oscillator.frequency.value = frequency;
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(volume, start + 0.012);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
      oscillator.connect(gain).connect(ctx.destination);
      oscillator.start(start);
      oscillator.stop(start + duration + 0.02);
      start += duration * 0.8;
    }
  } catch {
    // Audio feedback is optional; ignore devices without output.
  }
}

export const sounds = {
  start: () => tone([660, 880]),
  stop: () => tone([880, 660]),
  done: () => tone([1046], 0.07, 0.05),
  error: () => tone([330, 247], 0.14),
};
