export default function () {
  const bar = document.getElementById('loading-bar-fill');
  const status = document.getElementById('loading-status');

  return {
    onStart() {
      if (bar) {
        bar.classList.remove('indeterminate');
        bar.style.width = '0%';
      }
      if (status) {
        status.textContent = 'Downloading…';
        status.style.color = '#e9dab2';
      }
    },
    onProgress({ current, total }) {
      const curMb = (current / (1024 * 1024)).toFixed(1);
      if (total > 0) {
        const totMb = (total / (1024 * 1024)).toFixed(1);
        const pct = Math.min(100, Math.max(0, (current / total) * 100));
        if (bar) {
          bar.classList.remove('indeterminate');
          bar.style.width = `${pct}%`;
        }
        if (status) {
          status.textContent = `Downloading ${curMb} / ${totMb} MB`;
          status.style.color = '#e9dab2';
        }
      } else {
        if (bar) {
          bar.classList.add('indeterminate');
        }
        if (status) {
          status.textContent = `Downloading ${curMb} MB`;
          status.style.color = '#e9dab2';
        }
      }
    },
    onComplete() {},
    onSuccess(_wasm) {
      if (bar) {
        bar.classList.remove('indeterminate');
        bar.style.width = '100%';
      }
      if (status) {
        status.textContent = 'Starting…';
        status.style.color = '#e9dab2';
      }
      setTimeout(() => {
        if (status && status.textContent === 'Starting…') {
          status.textContent = 'Loading art…';
        }
      }, 0);
    },
    onFailure(error) {
      if (bar) {
        bar.classList.remove('indeterminate');
      }
      if (status) {
        const msg = (error && (error.message || error.toString())) || 'Unknown error';
        status.textContent = `Failed to load: ${msg}`;
        status.style.color = '#ff5555';
      }
    },
  };
}
