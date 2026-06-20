import init, { Solver } from './pkg/torpor.min.js';

const IS_WORKER = typeof DedicatedWorkerGlobalScope !== 'undefined' && self instanceof DedicatedWorkerGlobalScope;

if (IS_WORKER) {
  let ready;
  self.onmessage = async (event) => {
    if (event.data?.type !== 'solve') return;
    const { modulus, base, difficulty } = event.data;
    try {
      ready ??= init();
      await ready;
      const solver = new Solver(modulus, base, BigInt(difficulty));
      self.postMessage({ type: 'started' });

      let chunk = 2000;
      while (!solver.done) {
        const t0 = performance.now();
        solver.step(BigInt(chunk));
        const dt = performance.now() - t0;
        self.postMessage({ type: 'progress', progress: solver.progress });
        if (dt > 0) chunk = Math.min(Math.max(Math.round((chunk * 120) / dt), 1000), 5_000_000);
      }
      self.postMessage({ type: 'done', answer: solver.answerHex });
    } catch (err) {
      self.postMessage({ type: 'error', message: String(err?.message ?? err) });
    }
  };
}

export class Torpor {
  constructor(challenge) {
    this.challenge = challenge;
    this.onStart = null; //  () => void
    this.onProgress = null; // ({ progress, elapsedMs, rate, etaMs }) => void
    this._worker = null;
    this._blobUrl = null;
  }

  solve() {
    const { difficulty } = this.challenge;
    const shim = `import ${JSON.stringify(import.meta.url)};`;
    const blobUrl = URL.createObjectURL(new Blob([shim], { type: 'text/javascript' }));
    const worker = new Worker(blobUrl, { type: 'module' });
    this._worker = worker;
    this._blobUrl = blobUrl;
    const start = performance.now();

    return new Promise((resolve, reject) => {
      const finish = (settle, value) => {
        this.cancel();
        settle(value);
      };
      worker.onmessage = (event) => {
        const m = event.data;
        switch (m.type) {
          case 'started':
            this.onStart?.();
            break;
          case 'progress': {
            const elapsedMs = performance.now() - start;
            const rate = elapsedMs > 0 ? (m.progress * difficulty * 1000) / elapsedMs : 0;
            const etaMs = m.progress > 0 ? (elapsedMs * (1 - m.progress)) / m.progress : Infinity;
            this.onProgress?.({ progress: m.progress, elapsedMs, rate, etaMs });
            break;
          }
          case 'done':
            finish(resolve, { answer: m.answer, elapsedMs: performance.now() - start });
            break;
          case 'error':
            finish(reject, new Error(m.message));
            break;
        }
      };
      worker.onerror = (event) => finish(reject, event.error ?? new Error(event.message));
      worker.postMessage({ type: 'solve', ...this.challenge });
    });
  }

  cancel() {
    this._worker?.terminate();
    this._worker = null;
    if (this._blobUrl) {
      URL.revokeObjectURL(this._blobUrl);
      this._blobUrl = null;
    }
  }
}
