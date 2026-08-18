document.addEventListener('DOMContentLoaded', () => {
  const audioCtx = window.AudioContext || window.webkitAudioContext;
  const playTone = () => {
    if (!audioCtx) return;
    const ctx = new audioCtx();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = 'square';
    osc.frequency.value = 440;
    gain.gain.value = 0.015;
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.05);
  };

  document.querySelectorAll('.site-nav a, .site-button, .footer-link').forEach((node) => {
    node.addEventListener('click', playTone);
  });
});
