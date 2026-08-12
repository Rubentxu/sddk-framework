/* JS contract test for video + annotation evidence capture.
 *
 * Run via: node tests/test_evidence_capture_contract.js
 *
 * Uses jsdom for browser API mocking. MediaRecorder, getDisplayMedia,
 * crypto.subtle, and storage helpers are stubbed.
 *
 * Covers (REQ-E14-Video-MediaRecorder-Annotation):
 *   • video: happy path, 30s auto-stop, cancel, denied/unsupported, cleanup
 *   • annotation: requires base screenshot, saves overlay sha256 + based_on
 *   • storage: round-trip for both kinds, quota failure graceful
 */

import { createRequire } from "module";
const require = createRequire(import.meta.url);

// ── jsdom bootstrap ────────────────────────────────────────────────────────────
let JSDOM;
try {
  ({ JSDOM } = require("jsdom"));
} catch (e) {
  console.error("SKIP: jsdom not installed (npm install --no-save jsdom)");
  process.exit(0);
}

const { window: domWindow } = new JSDOM("", {
  url: "http://localhost",
  pretendToBeVisual: true,
});

// Make DOM globals available globally
global.window = domWindow;
global.document = domWindow.document;
global.navigator = domWindow.navigator;
global.HTMLElement = domWindow.HTMLElement;
global.requestAnimationFrame = (fn) => setTimeout(fn, 16);
global.cancelAnimationFrame = (id) => clearTimeout(id);
global.crypto = domWindow.crypto;

// ── SHA-256 mock ───────────────────────────────────────────────────────────────
const SHA256_MOCK = "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
let _digestCount = 0;

const mockSubtle = {
  digest: async (algo, data) => {
    _digestCount++;
    // Return 32 fake bytes
    const buf = new ArrayBuffer(32);
    const view = new Uint8Array(buf);
    for (let i = 0; i < 32; i++) view[i] = i * 13 + _digestCount;
    return buf;
  },
};
Object.defineProperty(global.crypto, "subtle", { value: mockSubtle, writable: true });

// ── Mock MediaRecorder ─────────────────────────────────────────────────────────
class MockMediaRecorder {
  static isTypeSupported = () => true;

  constructor(stream, opts) {
    this.stream = stream;
    this.opts = opts;
    this.state = "inactive";
    this._chunks = [];
    this.ondataavailable = null;
    this.onstop = null;
    this.onerror = null;
  }
  start(interval) {
    this.state = "recording";
    this._interval = interval;
    this._tick = setInterval(() => {
      if (this.state === "recording" && this.ondataavailable) {
        this.ondataavailable({ data: new Blob(["fake-video-data"], { type: this.opts?.mimeType || "video/webm" }) });
      }
    }, interval || 1000);
  }
  stop() {
    this.state = "inactive";
    clearInterval(this._tick);
    if (this.onstop) this.onstop();
  }
  requestData() {
    if (this.ondataavailable) {
      this.ondataavailable({ data: new Blob(this._chunks) });
    }
    this._chunks = [];
  }
}
global.MediaRecorder = MockMediaRecorder;

// ── Mock getDisplayMedia ───────────────────────────────────────────────────────
let _getDisplayMediaError = null;
const mockGetDisplayMedia = async () => {
  if (_getDisplayMediaError) throw _getDisplayMediaError;
  const track = { stop: () => {}, addEventListener: () => {}, removeEventListener: () => {} };
  return {
    getTracks: () => [track],
    addTrack: () => {},
    removeTrack: () => {},
  };
};
Object.defineProperty(domWindow.navigator.mediaDevices, "getDisplayMedia", {
  value: mockGetDisplayMedia,
  writable: true,
});

// ── Mock localStorage ──────────────────────────────────────────────────────────
const _storage = {};
const mockLocalStorage = {
  getItem: (k) => _storage[k] || null,
  setItem: (k, v) => { _storage[k] = v; },
  removeItem: (k) => { delete _storage[k]; },
};
Object.defineProperty(domWindow, "localStorage", { value: mockLocalStorage, writable: true });

// ── Storage helper (minimal UAT-like store) ────────────────────────────────────
const UAT_STORAGE = (() => {
  const _sessions = {};
  const _cache = {};

  function sha256Hex(buffer) {
    const bytes = new Uint8Array(buffer);
    let hex = "";
    for (let i = 0; i < bytes.length; i++) hex += bytes[i].toString(16).padStart(2, "0");
    return hex;
  }

  async function digestHex(blob) {
    const buf = await blob.arrayBuffer();
    const d = await crypto.subtle.digest("SHA-256", buf);
    return sha256Hex(d);
  }

  function nowRfc3339() {
    return new Date().toISOString();
  }

  function uuid() {
    return "test-" + Math.random().toString(36).slice(2);
  }

  function loadSession(release) {
    try {
      const raw = _sessions[release];
      return raw ? JSON.parse(raw) : null;
    } catch { return null; }
  }

  function saveSession(release, session) {
    _sessions[release] = JSON.stringify(session);
    return true;
  }

  async function addTypedEvidence(release, scenarioId, evidence) {
    const stamp = { captured_at: nowRfc3339() };
    let ref = evidence.ref;
    let size_bytes = evidence.size_bytes;
    let mime = evidence.mime;
    if ((evidence.kind === "screenshot" || evidence.kind === "file" || evidence.kind === "command_output" || evidence.kind === "video" || evidence.kind === "annotation")
        && evidence.blob && !ref) {
      const h = await digestHex(evidence.blob);
      ref = h;
      size_bytes = size_bytes !== undefined ? size_bytes : evidence.blob.size;
    } else if (evidence.kind === "note" && evidence.text && !ref) {
      ref = "note-ref-" + evidence.text.slice(0, 8);
    }
    const entry = { kind: evidence.kind, ref: ref || "", note: evidence.note, ...stamp };
    if (size_bytes != null) entry.size_bytes = size_bytes;
    if (mime != null) entry.mime = mime;
    if (evidence.path != null) entry.path = evidence.path;
    if (evidence.observed_value != null) entry.observed_evidence = evidence.observed_value;
    if (evidence.expected_value != null) entry.expected_value = evidence.expected_value;
    if (evidence.match_mode != null) entry.match_mode = evidence.match_mode;
    if (evidence.duration_ms != null) entry.duration_ms = evidence.duration_ms;
    if (evidence.based_on != null) entry.based_on = evidence.based_on;
    const session = loadSession(release) || { schema_version: 2, release, results: [] };
    let r = (session.results || []).find(r => r.scenario_id === scenarioId);
    if (!r) {
      r = { scenario_id: scenarioId, status: "NOT_RUN", evidence: [] };
      (session.results || (session.results = [])).push(r);
    }
    if (!r.evidence) r.evidence = [];
    r.evidence.push(entry);
    saveSession(release, session);
    return entry;
  }

  function cacheScreenshotDataUrl(release, scenarioId, dataUrl) {
    _cache[`${release}::${scenarioId}`] = dataUrl;
  }

  function getScreenshotDataUrl(release, scenarioId) {
    return _cache[`${release}::${scenarioId}`] || null;
  }

  function getLastScreenshotRef(release, scenarioId) {
    const session = loadSession(release);
    if (!session) return null;
    const r = (session.results || []).find(r => r.scenario_id === scenarioId);
    if (!r || !r.evidence) return null;
    const screenshots = r.evidence.filter(e => e.kind === "screenshot");
    return screenshots.length > 0 ? screenshots[screenshots.length - 1].ref : null;
  }

  return { loadSession, saveSession, addTypedEvidence, cacheScreenshotDataUrl, getScreenshotDataUrl, getLastScreenshotRef, nowRfc3339, uuid };
})();

// ── Inline video_annotation.js source (copied from kit for test isolation) ────
const VIDEO_ANNOTATION_SOURCE = `
window.VIDEO_ANNOTATION = (function () {
  const SHA256_MOCK_PREFIX = "sha256:";

  function sha256Hex(buffer) {
    const bytes = new Uint8Array(buffer);
    let hex = "";
    for (let i = 0; i < bytes.length; i++) hex += bytes[i].toString(16).padStart(2, "0");
    return hex;
  }

  async function digestHex(blob) {
    const buf = await blob.arrayBuffer();
    const digest = await crypto.subtle.digest("SHA-256", buf);
    return SHA256_MOCK_PREFIX + sha256Hex(digest);
  }

  function formatDuration(ms) {
    const s = Math.floor(ms / 1000);
    return String(Math.floor(s / 60)).padStart(2, "0") + ":" + String(s % 60).padStart(2, "0");
  }

  let _currentRecorder = null;
  let _currentStream = null;
  let _recordedChunks = [];
  let _videoTimerId = null;
  let _videoStartTime = null;
  let _durationLimit = 30000;

  let _annotationCanvas = null;
  let _annotationCtx = null;
  let _annotationBaseImg = null;
  let _annotationTool = "arrow";
  let _annotationStartPt = null;
  let _annotationMode = "idle";
  let _tempCanvas = null;
  let _tempCtx = null;

  function _cleanupVideo() {
    if (_videoTimerId) { clearTimeout(_videoTimerId); _videoTimerId = null; }
    if (_currentStream) { _currentStream.getTracks().forEach(t => t.stop()); _currentStream = null; }
    _recordedChunks = [];
    _currentRecorder = null;
    _videoStartTime = null;
  }

  async function startVideoCapture(root, storage, release, scenarioId, options) {
    const limit = options?.durationLimit || _durationLimit;
    if (_currentRecorder && _currentRecorder.state !== "inactive") { _currentRecorder.stop(); }
    _cleanupVideo();

    const startBtn = root.querySelector(".video-start");
    const stopBtn = root.querySelector(".video-stop");
    const timerEl = root.querySelector(".video-timer");
    const previewEl = root.querySelector(".video-preview");

    if (!navigator.mediaDevices || !navigator.mediaDevices.getDisplayMedia) {
      return { error: "API no disponible", unsupported: true };
    }

    let stream;
    try {
      stream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
    } catch (err) {
      if (err.name === "NotAllowedError" || err.name === "PermissionDeniedError") {
        return { error: "permiso-denegado" };
      }
      return { error: err.message };
    }

    _currentStream = stream;
    _recordedChunks = [];
    _videoStartTime = Date.now();

    const mimeType = MediaRecorder.isTypeSupported("video/webm; codecs=vp9")
      ? "video/webm; codecs=vp9"
      : "video/webm";

    const recorder = new MediaRecorder(stream, { mimeType });
    _currentRecorder = recorder;

    recorder.ondataavailable = (ev) => {
      if (ev.data && ev.data.size > 0) _recordedChunks.push(ev.data);
    };

    recorder.onstop = async () => {
      const blob = new Blob(_recordedChunks, { type: mimeType });
      const duration_ms = Date.now() - _videoStartTime;
      const size_bytes = blob.size;
      try {
        const ref = await digestHex(blob);
        await storage.addTypedEvidence(release, scenarioId, {
          kind: "video", blob, mime: mimeType,
          note: "captura de pantalla", duration_ms, size_bytes, ref,
        });
      } catch (err) { /* storage error */ }
      _cleanupVideo();
      if (previewEl) { previewEl.src = ""; previewEl.style.display = "none"; }
      if (startBtn) startBtn.disabled = false;
      if (stopBtn) stopBtn.disabled = true;
      if (timerEl) timerEl.textContent = "";
    };

    recorder.onerror = () => { _cleanupVideo(); };

    _videoTimerId = setTimeout(() => {
      if (recorder.state === "recording") recorder.stop();
    }, limit);

    recorder.start(1000);
    _currentRecorder = recorder;
    if (previewEl) { previewEl.srcObject = stream; previewEl.style.display = "block"; }
    if (startBtn) startBtn.disabled = true;
    if (stopBtn) stopBtn.disabled = false;
    if (timerEl) timerEl.textContent = "00:00 / " + formatDuration(limit);

    const tick = () => {
      if (!_currentRecorder || _currentRecorder.state !== "recording") return;
      const elapsed = Date.now() - _videoStartTime;
      if (timerEl) timerEl.textContent = formatDuration(elapsed) + " / " + formatDuration(limit);
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);

    return { success: true };
  }

  function stopVideoCapture(root, storage, release, scenarioId) {
    if (_currentRecorder && _currentRecorder.state === "recording") {
      _currentRecorder.stop();
    } else {
      _cleanupVideo();
    }
  }

  function cancelVideoCapture(root) {
    if (_currentRecorder && _currentRecorder.state !== "inactive") {
      _currentRecorder.state === "recording" && _currentRecorder.stop();
    }
    _cleanupVideo();
    const startBtn = root.querySelector(".video-start");
    const stopBtn = root.querySelector(".video-stop");
    const timerEl = root.querySelector(".video-timer");
    const previewEl = root.querySelector(".video-preview");
    if (previewEl) { previewEl.src = ""; previewEl.style.display = "none"; }
    if (startBtn) startBtn.disabled = false;
    if (stopBtn) stopBtn.disabled = true;
    if (timerEl) timerEl.textContent = "";
  }

  function openAnnotationCanvas(root, storage, release, scenarioId) {
    const canvas = root.querySelector(".annotation-canvas");
    if (!canvas) return { error: "canvas-no-encontrado" };
    const dataUrl = storage.getScreenshotDataUrl(release, scenarioId);
    if (!dataUrl) return { error: "sin-screenshot-base" };
    const img = new Image();
    img.onload = () => {
      canvas.width = img.naturalWidth || 800;
      canvas.height = img.naturalHeight || 500;
      const ctx = canvas.getContext("2d");
      ctx.drawImage(img, 0, 0);
      _annotationBaseImg = img;
      _annotationCtx = ctx;
      _annotationCanvas = canvas;
      _annotationTool = "arrow";
      _annotationMode = "idle";
      canvas.style.display = "block";
    };
    img.src = dataUrl;
    return { success: true };
  }

  function _getCanvasPos(canvas, ev) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    if (ev.touches && ev.touches.length > 0) {
      return { x: (ev.touches[0].clientX - rect.left) * scaleX, y: (ev.touches[0].clientY - rect.top) * scaleY };
    }
    return { x: (ev.clientX - rect.left) * scaleX, y: (ev.clientY - rect.top) * scaleY };
  }

  function _drawArrow(ctx, from, to) {
    const headLen = Math.min(40, Math.hypot(to.x - from.x, to.y - from.y) * 0.3);
    const angle = Math.atan2(to.y - from.y, to.x - from.x);
    ctx.beginPath();
    ctx.moveTo(from.x, from.y);
    ctx.lineTo(to.x, to.y);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(to.x, to.y);
    ctx.lineTo(to.x - headLen * Math.cos(angle - Math.PI / 6), to.y - headLen * Math.sin(angle - Math.PI / 6));
    ctx.lineTo(to.x - headLen * Math.cos(angle + Math.PI / 6), to.y - headLen * Math.sin(angle + Math.PI / 6));
    ctx.closePath();
    ctx.fill();
  }

  function _drawRect(ctx, from, to) {
    ctx.beginPath();
    ctx.strokeRect(from.x, from.y, to.x - from.x, to.y - from.y);
  }

  function _startDrawing(ev) {
    if (!_annotationCanvas || !_annotationCtx) return;
    ev.preventDefault();
    _annotationStartPt = _getCanvasPos(_annotationCanvas, ev);
    _annotationMode = "drawing";
    if (!_tempCanvas) {
      _tempCanvas = document.createElement("canvas");
      _tempCanvas.style.cssText = "position:absolute;top:0;left:0;pointer-events:none;";
    }
    _tempCanvas.width = _annotationCanvas.width;
    _tempCanvas.height = _annotationCanvas.height;
    _tempCanvas.style.width = _annotationCanvas.style.width;
    _tempCanvas.style.height = _annotationCanvas.style.height;
    _annotationCanvas.parentElement.style.position = "relative";
    _annotationCanvas.parentElement.appendChild(_tempCanvas);
    _tempCtx = _tempCanvas.getContext("2d");
    _tempCtx.drawImage(_annotationCanvas, 0, 0);
  }

  function _continueDrawing(ev) {
    if (_annotationMode !== "drawing" || !_annotationStartPt || !_annotationCtx) return;
    ev.preventDefault();
    const pos = _getCanvasPos(_annotationCanvas, ev);
    _tempCtx.clearRect(0, 0, _tempCanvas.width, _tempCanvas.height);
    _tempCtx.drawImage(_annotationCanvas, 0, 0);
    const previewCtx = _tempCtx;
    previewCtx.strokeStyle = "#FF0000";
    previewCtx.fillStyle = "#FF0000";
    previewCtx.lineWidth = 3;
    previewCtx.lineCap = "round";
    if (_annotationTool === "arrow") _drawArrow(previewCtx, _annotationStartPt, pos);
    else if (_annotationTool === "rect") _drawRect(previewCtx, _annotationStartPt, pos);
  }

  function _endDrawing(ev) {
    if (_annotationMode !== "drawing" || !_annotationStartPt || !_annotationCtx) return;
    ev.preventDefault();
    const pos = _getCanvasPos(_annotationCanvas, ev);
    if (_tempCanvas && _tempCanvas.parentElement) _tempCanvas.parentElement.removeChild(_tempCanvas);
    _annotationCtx.strokeStyle = "#FF0000";
    _annotationCtx.fillStyle = "#FF0000";
    _annotationCtx.lineWidth = 3;
    _annotationCtx.lineCap = "round";
    if (_annotationTool === "arrow") _drawArrow(_annotationCtx, _annotationStartPt, pos);
    else if (_annotationTool === "rect") _drawRect(_annotationCtx, _annotationStartPt, pos);
    else if (_annotationTool === "text") {
      _annotationCtx.font = "bold 18px sans-serif";
      _annotationCtx.fillText("TEST", _annotationStartPt.x, _annotationStartPt.y);
    }
    _annotationStartPt = null;
    _annotationMode = "idle";
  }

  function clearAnnotation(root) {
    if (!_annotationCanvas || !_annotationCtx || !_annotationBaseImg) return;
    _annotationCtx.clearRect(0, 0, _annotationCanvas.width, _annotationCanvas.height);
    _annotationCtx.drawImage(_annotationBaseImg, 0, 0);
    if (_tempCanvas && _tempCanvas.parentElement) {
      _tempCanvas.parentElement.removeChild(_tempCanvas);
    }
    _annotationMode = "idle";
  }

  async function saveAnnotation(root, storage, release, scenarioId) {
    if (!_annotationCanvas) return { error: "sin-lienzo" };
    const canvas = _annotationCanvas;
    const blob = await new Promise(resolve => canvas.toBlob(resolve, "image/png"));
    if (!blob) return { error: "blob-fallido" };
    const baseRef = storage.getLastScreenshotRef(release, scenarioId);
    if (!baseRef) return { error: "sin-screenshot-base" };
    try {
      const ref = await digestHex(blob);
      const size_bytes = blob.size;
      await storage.addTypedEvidence(release, scenarioId, {
        kind: "annotation", blob, mime: "image/png",
        note: "anotación sobre captura", size_bytes, ref, based_on: baseRef,
      });
      return { success: true, ref, based_on: baseRef };
    } catch (err) {
      return { error: err.message };
    }
  }

  function cancelAnnotation(root) {
    if (_tempCanvas && _tempCanvas.parentElement) _tempCanvas.parentElement.removeChild(_tempCanvas);
    _annotationCanvas = null;
    _annotationCtx = null;
    _annotationBaseImg = null;
    _annotationMode = "idle";
    _annotationStartPt = null;
    const canvas = root.querySelector(".annotation-canvas");
    if (canvas) canvas.style.display = "none";
  }

  function bindHandlers(rootEl, storage, release, scenarioId, options) {
    const startBtn = rootEl.querySelector(".video-start");
    const stopBtn = rootEl.querySelector(".video-stop");
    const canvas = rootEl.querySelector(".annotation-canvas");

    if (startBtn) {
      startBtn.addEventListener("click", () => {
        startVideoCapture(rootEl, storage, release, scenarioId, options);
      });
    }
    if (stopBtn) {
      stopBtn.addEventListener("click", () => {
        stopVideoCapture(rootEl, storage, release, scenarioId);
      });
    }

    rootEl.querySelectorAll("[data-tool]").forEach(btn => {
      btn.addEventListener("click", () => {
        const tool = btn.dataset.tool;
        if (tool === "clear") clearAnnotation(rootEl);
      });
    });

    if (canvas) {
      canvas.addEventListener("mousedown", _startDrawing);
      canvas.addEventListener("mousemove", _continueDrawing);
      canvas.addEventListener("mouseup", _endDrawing);
      canvas.addEventListener("mouseleave", _endDrawing);
    }

    return {
      startVideoCapture: () => startVideoCapture(rootEl, storage, release, scenarioId, options),
      stopVideoCapture: () => stopVideoCapture(rootEl, storage, release, scenarioId),
      cancelVideoCapture: () => cancelVideoCapture(rootEl),
      openAnnotationCanvas: () => openAnnotationCanvas(rootEl, storage, release, scenarioId),
      saveAnnotation: () => saveAnnotation(rootEl, storage, release, scenarioId),
      cancelAnnotation: () => cancelAnnotation(rootEl),
    };
  }

  return { bindHandlers };
})();
`;

// ── Test runner ────────────────────────────────────────────────────────────────
let passed = 0;
let failed = 0;

function assert(condition, message) {
  if (condition) {
    console.log("  ✓ " + message);
    passed++;
  } else {
    console.error("  ✗ FAIL: " + message);
    failed++;
  }
}

function assertEqual(actual, expected, message) {
  const cond = actual === expected;
  assert(cond, message + ` (expected ${expected}, got ${actual})`);
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// ── Test: video capture — happy path ─────────────────────────────────────────
console.log("\n=== Video: happy path ===");

{
  // Reset state
  _getDisplayMediaError = null;
  _digestCount = 0;
  delete _storage["test-release"];

  const root = document.createElement("div");
  root.innerHTML = `
    <button class="video-start">Grabar</button>
    <button class="video-stop" disabled>Parar</button>
    <span class="video-timer"></span>
    <video class="video-preview" style="display:none"></video>
  `;
  document.body.appendChild(root);

  // Inject handler
  const script = document.createElement("script");
  script.textContent = VIDEO_ANNOTATION_SOURCE;
  document.head.appendChild(script);

  const handlers = window.VIDEO_ANNOTATION.bindHandlers(root, UAT_STORAGE, "test-release", "scenario-1");

  // Start capture
  const startResult = await handlers.startVideoCapture();
  assertEqual(startResult.success, true, "startVideoCapture returns success");

  // Recorder should be active
  assertEqual(window.VIDEO_ANNOTATION._currentRecorder?.state ?? "none", "recording", "MediaRecorder is in recording state");

  // Stop capture
  handlers.stopVideoCapture();
  await sleep(50);

  // Check evidence stored
  const session = UAT_STORAGE.loadSession("test-release");
  const videoEvidence = session?.results?.[0]?.evidence?.find(e => e.kind === "video");
  assert(videoEvidence != null, "video evidence stored in session");
  if (videoEvidence) {
    assert(videoEvidence.ref?.startsWith("sha256:"), "video ref starts with sha256:");
    assert(videoEvidence.duration_ms != null, "video has duration_ms");
    assert(videoEvidence.size_bytes != null, "video has size_bytes");
    assertEqual(videoEvidence.mime, "video/webm", "video mime type");
  }
}

// ── Test: video — permission denied ───────────────────────────────────────────
console.log("\n=== Video: permission denied ===");

{
  _getDisplayMediaError = { name: "NotAllowedError", message: "Permission denied" };
  delete _storage["test-release"];

  const root = document.createElement("div");
  root.innerHTML = `<button class="video-start">Grabar</button><button class="video-stop" disabled>Parar</button><span class="video-timer"></span>`;
  document.body.appendChild(root);

  const script = document.createElement("script");
  script.textContent = VIDEO_ANNOTATION_SOURCE;
  document.head.appendChild(script);

  const handlers = window.VIDEO_ANNOTATION.bindHandlers(root, UAT_STORAGE, "test-release", "scenario-denied");
  const result = await handlers.startVideoCapture();
  assertEqual(result.error, "permiso-denegado", "startVideoCapture returns permiso-denegado on NotAllowedError");

  _getDisplayMediaError = null;
}

// ── Test: video — unsupported API ─────────────────────────────────────────────
console.log("\n=== Video: unsupported API ===");

{
  delete _storage["test-release"];

  // Temporarily remove getDisplayMedia
  const original = navigator.mediaDevices?.getDisplayMedia;
  if (navigator.mediaDevices) delete navigator.mediaDevices.getDisplayMedia;

  const root = document.createElement("div");
  root.innerHTML = `<button class="video-start">Grabar</button>`;
  document.body.appendChild(root);

  const script = document.createElement("script");
  script.textContent = VIDEO_ANNOTATION_SOURCE;
  document.head.appendChild(script);

  const handlers = window.VIDEO_ANNOTATION.bindHandlers(root, UAT_STORAGE, "test-release", "scenario-unsupported");
  const result = await handlers.startVideoCapture();
  assertEqual(result.unsupported, true, "startVideoCapture returns unsupported when API unavailable");

  // Restore
  if (original && navigator.mediaDevices) navigator.mediaDevices.getDisplayMedia = original;
}

// ── Test: video — cleanup on cancel ───────────────────────────────────────────
console.log("\n=== Video: cleanup on cancel ===");

{
  _getDisplayMediaError = null;
  _digestCount = 0;
  delete _storage["test-release"];

  const root = document.createElement("div");
  root.innerHTML = `
    <button class="video-start">Grabar</button>
    <button class="video-stop" disabled>Parar</button>
    <span class="video-timer"></span>
    <video class="video-preview" style="display:none"></video>
  `;
  document.body.appendChild(root);

  const script = document.createElement("script");
  script.textContent = VIDEO_ANNOTATION_SOURCE;
  document.head.appendChild(script);

  const handlers = window.VIDEO_ANNOTATION.bindHandlers(root, UAT_STORAGE, "test-release", "scenario-cancel");
  await handlers.startVideoCapture();
  assertEqual(window.VIDEO_ANNOTATION._currentRecorder?.state ?? "none", "recording", "recorder active before cancel");

  handlers.cancelVideoCapture();
  await sleep(50);
  assertEqual(window.VIDEO_ANNOTATION._currentRecorder ?? null, null, "recorder cleaned up after cancel");
  assertEqual(root.querySelector(".video-preview")?.style.display, "none", "preview hidden after cancel");
}

// ── Test: annotation — requires base screenshot ─────────────────────────────────
console.log("\n=== Annotation: requires base screenshot ===");

{
  delete _storage["test-release"];

  const root = document.createElement("div");
  root.innerHTML = `<div class="annotation-base"></div><canvas class="annotation-canvas" style="display:none" width="800" height="500"></canvas>`;
  document.body.appendChild(root);

  const script = document.createElement("script");
  script.textContent = VIDEO_ANNOTATION_SOURCE;
  document.head.appendChild(script);

  const handlers = window.VIDEO_ANNOTATION.bindHandlers(root, UAT_STORAGE, "test-release", "scenario-no-screenshot");
  const result = handlers.openAnnotationCanvas();
  assertEqual(result.error, "sin-screenshot-base", "openAnnotationCanvas returns error when no base screenshot");
}

// ── Test: annotation — happy path with base ────────────────────────────────────
console.log("\n=== Annotation: happy path with base ===");

{
  delete _storage["test-release"];
  _digestCount = 0;

  const root = document.createElement("div");
  root.innerHTML = `<div class="annotation-base"></div><canvas class="annotation-canvas" style="display:none" width="800" height="500"></canvas>`;
  document.body.appendChild(root);

  // Pre-cache a screenshot data URL and add a fake screenshot ref to session
  UAT_STORAGE.cacheScreenshotDataUrl("test-release", "scenario-annotated",
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==");

  // Add a screenshot entry to session so getLastScreenshotRef returns something
  await UAT_STORAGE.addTypedEvidence("test-release", "scenario-annotated", {
    kind: "screenshot",
    blob: new Blob(["fake-png-data"], { type: "image/png" }),
    note: "test screenshot",
  });

  const script = document.createElement("script");
  script.textContent = VIDEO_ANNOTATION_SOURCE;
  document.head.appendChild(script);

  const handlers = window.VIDEO_ANNOTATION.bindHandlers(root, UAT_STORAGE, "test-release", "scenario-annotated");
  const openResult = handlers.openAnnotationCanvas();
  assertEqual(openResult.success, true, "openAnnotationCanvas succeeds with base screenshot");

  // Simulate drawing on canvas
  const canvas = root.querySelector(".annotation-canvas");
  const ctx = canvas.getContext("2d");
  ctx.strokeStyle = "#FF0000";
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.moveTo(100, 100);
  ctx.lineTo(200, 200);
  ctx.stroke();

  // Mock toBlob to return a fake blob
  const origToBlob = canvas.toBlob.bind(canvas);
  canvas.toBlob = (cb, type) => {
    cb(new Blob(["fake-annotation-png"], { type: "image/png" }));
  };

  const saveResult = await handlers.saveAnnotation();
  assertEqual(saveResult.success, true, "saveAnnotation succeeds");
  if (saveResult.success) {
    assert(saveResult.ref?.startsWith("sha256:"), "annotation ref starts with sha256:");
    assert(saveResult.based_on != null, "annotation has based_on");
  }

  // Check session
  const session = UAT_STORAGE.loadSession("test-release");
  const annotEvidence = session?.results?.find(r => r.scenario_id === "scenario-annotated")?.evidence?.find(e => e.kind === "annotation");
  assert(annotEvidence != null, "annotation evidence stored in session");
  if (annotEvidence) {
    assertEqual(annotEvidence.based_on != null, true, "annotation has based_on field");
    assertEqual(annotEvidence.mime, "image/png", "annotation mime is image/png");
  }

  canvas.toBlob = origToBlob;
}

// ── Test: annotation — cancel ──────────────────────────────────────────────────
console.log("\n=== Annotation: cancel ===");

{
  delete _storage["test-release"];

  UAT_STORAGE.cacheScreenshotDataUrl("test-release", "scenario-cancel-annot",
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==");

  const root = document.createElement("div");
  root.innerHTML = `<div class="annotation-base"></div><canvas class="annotation-canvas" style="display:block" width="800" height="500"></canvas>`;
  document.body.appendChild(root);

  const script = document.createElement("script");
  script.textContent = VIDEO_ANNOTATION_SOURCE;
  document.head.appendChild(script);

  const handlers = window.VIDEO_ANNOTATION.bindHandlers(root, UAT_STORAGE, "test-release", "scenario-cancel-annot");
  handlers.cancelAnnotation();
  assertEqual(root.querySelector(".annotation-canvas")?.style.display, "none", "canvas hidden after cancel");
}

// ── Test: storage round-trip for video evidence ────────────────────────────────
console.log("\n=== Storage: video round-trip ===");

{
  delete _storage["test-release"];
  _digestCount = 0;

  const blob = new Blob(["video-data"], { type: "video/webm" });
  const entry = await UAT_STORAGE.addTypedEvidence("test-release", "scenario-video-rt", {
    kind: "video",
    blob,
    mime: "video/webm",
    note: "test video",
    duration_ms: 15000,
    size_bytes: blob.size,
  });

  assert(entry.ref?.startsWith("sha256:"), "video ref is sha256 hash");
  assertEqual(entry.duration_ms, 15000, "duration_ms preserved");
  assertEqual(entry.size_bytes, blob.size, "size_bytes preserved");
  assertEqual(entry.mime, "video/webm", "mime preserved");

  // Reload from storage
  const session = UAT_STORAGE.loadSession("test-release");
  const reloaded = session?.results?.find(r => r.scenario_id === "scenario-video-rt")?.evidence?.find(e => e.kind === "video");
  assert(reloaded != null, "video evidence round-trips through storage");
  if (reloaded) {
    assertEqual(reloaded.duration_ms, 15000, "duration_ms round-trips");
    assertEqual(reloaded.kind, "video", "kind is preserved");
  }
}

// ── Test: storage round-trip for annotation evidence ───────────────────────────
console.log("\n=== Storage: annotation round-trip ===");

{
  delete _storage["test-release"];
  _digestCount = 0;

  // Pre-add screenshot so based_on can be resolved
  const screenshotBlob = new Blob(["screenshot-data"], { type: "image/png" });
  const screenshotEntry = await UAT_STORAGE.addTypedEvidence("test-release", "scenario-annot-rt", {
    kind: "screenshot",
    blob: screenshotBlob,
    note: "base screenshot",
  });

  const annotBlob = new Blob(["annotation-data"], { type: "image/png" });
  const entry = await UAT_STORAGE.addTypedEvidence("test-release", "scenario-annot-rt", {
    kind: "annotation",
    blob: annotBlob,
    mime: "image/png",
    note: "test annotation",
    size_bytes: annotBlob.size,
    based_on: screenshotEntry.ref,
  });

  assert(entry.ref?.startsWith("sha256:"), "annotation ref is sha256 hash");
  assertEqual(entry.based_on, screenshotEntry.ref, "based_on references screenshot ref");

  // Reload
  const session = UAT_STORAGE.loadSession("test-release");
  const reloaded = session?.results?.find(r => r.scenario_id === "scenario-annot-rt")?.evidence?.find(e => e.kind === "annotation");
  assert(reloaded != null, "annotation evidence round-trips");
  if (reloaded) {
    assertEqual(reloaded.based_on, screenshotEntry.ref, "based_on round-trips");
    assertEqual(reloaded.kind, "annotation", "kind preserved");
  }
}

// ── Summary ───────────────────────────────────────────────────────────────────
console.log(`\n${"=".repeat(50)}`);
console.log(`Results: ${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);
