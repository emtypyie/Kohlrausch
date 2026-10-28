(function () {
  "use strict";

  const durationMs = 5000;
  const startedAt = performance.now();
  const track = document.querySelector(".progress-track");
  const fill = document.querySelector(".progress-fill");

  const timer = window.setInterval(() => {
    const progress = Math.min(100, Math.floor(((performance.now() - startedAt) / durationMs) * 100));
    fill.style.width = progress + "%";
    track.setAttribute("aria-valuenow", String(progress));
    if (progress >= 100) window.clearInterval(timer);
  }, 50);
})();
