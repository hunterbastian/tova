export function createAudioSystem() {
  let ctx = null;
  let master = null;
  let noise = null;
  let windState = null;

  function ensureContext() {
    if (!ctx) {
      ctx = new (window.AudioContext || window.webkitAudioContext)();
      master = ctx.createGain();
      master.gain.value = 0.55;
      master.connect(ctx.destination);

      /* Pre-generate 2 seconds of white noise, reused by all burst sounds */
      const len = ctx.sampleRate * 2;
      noise = ctx.createBuffer(1, len, ctx.sampleRate);
      const ch = noise.getChannelData(0);
      for (let i = 0; i < len; i++) ch[i] = Math.random() * 2 - 1;
    }

    if (ctx.state === "suspended") ctx.resume();
    return ctx;
  }

  /* ── helpers ────────────────────────────────────────── */

  function noiseBurst(freq, q, vol, attack, decay, type = "bandpass") {
    const ac = ensureContext();
    const t = ac.currentTime;
    const dur = attack + decay;

    const src = ac.createBufferSource();
    src.buffer = noise;

    const flt = ac.createBiquadFilter();
    flt.type = type;
    flt.frequency.value = freq;
    flt.Q.value = q;

    const g = ac.createGain();
    g.gain.setValueAtTime(0.0001, t);
    g.gain.linearRampToValueAtTime(vol, t + attack);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);

    src.connect(flt).connect(g).connect(master);
    src.start(t);
    src.stop(t + dur + 0.02);
  }

  function toneBurst(freq, endFreq, type, vol, dur) {
    const ac = ensureContext();
    const t = ac.currentTime;

    const osc = ac.createOscillator();
    osc.type = type;
    osc.frequency.setValueAtTime(freq, t);
    if (endFreq !== freq) {
      osc.frequency.exponentialRampToValueAtTime(endFreq, t + dur);
    }

    const g = ac.createGain();
    g.gain.setValueAtTime(vol, t);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);

    osc.connect(g).connect(master);
    osc.start(t);
    osc.stop(t + dur + 0.02);
  }

  /* ── sound effects ─────────────────────────────────── */

  function footstep(sprinting) {
    const baseFreq = sprinting ? 420 : 300;
    const freq = baseFreq + (Math.random() - 0.5) * 100;
    const vol = (sprinting ? 0.065 : 0.038) + Math.random() * 0.012;
    noiseBurst(freq, 1.0, vol, 0.006, 0.088);
  }

  function land(intensity) {
    const vol = 0.04 + Math.min(intensity, 1) * 0.06;
    noiseBurst(180, 0.7, vol, 0.004, 0.11, "lowpass");
    toneBurst(75, 32, "sine", vol * 1.1, 0.13);
  }

  function swordSwing() {
    const ac = ensureContext();
    const t = ac.currentTime;

    const src = ac.createBufferSource();
    src.buffer = noise;

    const flt = ac.createBiquadFilter();
    flt.type = "bandpass";
    flt.frequency.setValueAtTime(200, t);
    flt.frequency.exponentialRampToValueAtTime(900, t + 0.12);
    flt.frequency.exponentialRampToValueAtTime(350, t + 0.25);
    flt.Q.value = 2.2;

    const g = ac.createGain();
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(0.13, t + 0.055);
    g.gain.exponentialRampToValueAtTime(0.0001, t + 0.25);

    src.connect(flt).connect(g).connect(master);
    src.start(t);
    src.stop(t + 0.27);
  }

  function hit() {
    toneBurst(130, 40, "sine", 0.2, 0.16);
    noiseBurst(600, 1.4, 0.1, 0.003, 0.055);
  }

  function playerDamage() {
    const ac = ensureContext();
    const t = ac.currentTime;

    const osc = ac.createOscillator();
    osc.type = "sawtooth";
    osc.frequency.setValueAtTime(90, t);
    osc.frequency.exponentialRampToValueAtTime(28, t + 0.28);

    const flt = ac.createBiquadFilter();
    flt.type = "lowpass";
    flt.frequency.value = 320;

    const g = ac.createGain();
    g.gain.setValueAtTime(0.12, t);
    g.gain.exponentialRampToValueAtTime(0.0001, t + 0.32);

    osc.connect(flt).connect(g).connect(master);
    osc.start(t);
    osc.stop(t + 0.35);

    noiseBurst(220, 0.6, 0.07, 0.003, 0.09, "lowpass");
  }

  function pickup() {
    const ac = ensureContext();
    const t = ac.currentTime;

    const harmonics = [[520, 0.07], [780, 0.045]];
    for (const [freq, vol] of harmonics) {
      const osc = ac.createOscillator();
      osc.type = "sine";
      osc.frequency.setValueAtTime(freq, t);
      osc.frequency.exponentialRampToValueAtTime(freq * 1.12, t + 0.32);

      const g = ac.createGain();
      g.gain.setValueAtTime(0.0001, t);
      g.gain.linearRampToValueAtTime(vol, t + 0.018);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.38);

      osc.connect(g).connect(master);
      osc.start(t);
      osc.stop(t + 0.4);
    }
  }

  function startWind() {
    if (windState) return;
    const ac = ensureContext();

    /* stereo noise for spatial width */
    const len = ac.sampleRate * 4;
    const buf = ac.createBuffer(2, len, ac.sampleRate);
    for (let ch = 0; ch < 2; ch++) {
      const d = buf.getChannelData(ch);
      for (let i = 0; i < len; i++) d[i] = Math.random() * 2 - 1;
    }

    const src = ac.createBufferSource();
    src.buffer = buf;
    src.loop = true;

    const flt = ac.createBiquadFilter();
    flt.type = "lowpass";
    flt.frequency.value = 240;
    flt.Q.value = 0.5;

    /* LFO modulates filter cutoff for natural gusting */
    const lfo = ac.createOscillator();
    lfo.type = "sine";
    lfo.frequency.value = 0.13;

    const lfoDepth = ac.createGain();
    lfoDepth.gain.value = 110;
    lfo.connect(lfoDepth).connect(flt.frequency);

    const g = ac.createGain();
    g.gain.value = 0.028;

    src.connect(flt).connect(g).connect(master);
    src.start();
    lfo.start();

    windState = { src, lfo };
  }

  function stopWind() {
    if (!windState) return;
    windState.src.stop();
    windState.lfo.stop();
    windState = null;
  }

  return { footstep, land, swordSwing, hit, playerDamage, pickup, startWind, stopWind };
}
