/* Storage helpers: localStorage persistence + export/import JSON (zero backend).
 *
 * The exported JSON is the canonical `UatSession` shape consumed by
 * `sddk uat ingest` (crates/sddk-cli/src/uat.rs):
 *   schema_version, session_id, plan_ref, release, executor, executed_by,
 *   started_at, finished_at, results: [{ scenario_id, status, comment,
 *   evidence: [{ kind, ref, note }], duration_minutes }].
 *
 * Keeping the localStorage shape and the export shape in sync is the whole
 * point of this module: a tester hits "Finalizar y exportar" and gets a file
 * that drops straight into the CLI.
 */

const UAT = (() => {
  const KEY = (release) => `sddk-${release}`;

  function nowRfc3339() {
    return new Date().toISOString();
  }

  function uuid() {
    // Cheap UUID v4-ish (crypto.randomUUID when available, fallback otherwise).
    if (typeof crypto !== "undefined" && crypto.randomUUID) return crypto.randomUUID();
    return "uat-" + Math.random().toString(36).slice(2) + "-" + Date.now().toString(36);
  }

  function loadSession(release) {
    try {
      const raw = localStorage.getItem(KEY(release));
      return raw ? JSON.parse(raw) : null;
    } catch (e) { return null; }
  }

  function saveSession(release, session) {
    try {
      localStorage.setItem(KEY(release), JSON.stringify(session));
      return true;
    } catch (e) { return false; }
  }

  // Legacy formats the wizard may have written before canonicalization.
  function fromLegacy(legacy, plan) {
    if (!legacy) return null;
    if (Array.isArray(legacy.results)) return legacy; // already canonical
    if (legacy.scenario_results && plan) {
      const order = [];
      for (const f of plan.features || []) {
        for (const s of f.scenarios || []) order.push(s.id);
      }
      const results = [];
      for (const id of order) {
        const r = legacy.scenario_results[id];
        if (!r || !r.status) continue;
        results.push({
          scenario_id: id,
          status: r.status,
          comment: r.comment || "",
          evidence: r.evidence || [],
          duration_minutes: r.duration_minutes || 0,
        });
      }
      return {
        schema_version: 1,
        session_id: legacy.session_id || uuid(),
        plan_ref: legacy.plan_ref || plan.release?.candidate || "",
        release: legacy.release || "",
        executor: legacy.executor || "human",
        executed_by: legacy.executed_by || "",
        started_at: legacy.started_at || nowRfc3339(),
        finished_at: legacy.finished_at || null,
        results,
      };
    }
    return null;
  }

  // Build the canonical `UatSession` from the wizard's internal verdicts.
  function buildUatSession({ release, planRef, executedBy, startedAt, verdicts, scenarioOrder, finishedAt }) {
    const results = [];
    for (const id of scenarioOrder) {
      const v = verdicts[id];
      if (!v || !v.status) continue;
      results.push({
        scenario_id: id,
        status: v.status,
        comment: v.comment || "",
        evidence: v.evidence || [],
        duration_minutes: v.duration_minutes || 0,
      });
    }
    return {
      schema_version: 1,
      session_id: uuid(),
      plan_ref: planRef || release,
      release,
      executor: "human",
      executed_by: executedBy || "tester",
      started_at: startedAt || nowRfc3339(),
      finished_at: finishedAt || nowRfc3339(),
      results,
    };
  }

  function downloadBlob(filename, json) {
    const blob = new Blob([JSON.stringify(json, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  // Finalize the wizard: compose canonical UatSession, persist, download.
  function finalizeAndExport({ release, planRef, executedBy, startedAt, verdicts, scenarioOrder }) {
    const session = buildUatSession({
      release, planRef, executedBy, startedAt, verdicts, scenarioOrder,
      finishedAt: nowRfc3339(),
    });
    saveSession(release, session);
    downloadBlob(`uat-session-${release}.json`, session);
    return session;
  }

  function exportSession(release) {
    const session = loadSession(release);
    if (!session) return null;
    downloadBlob(`uat-session-${release}.json`, session);
    return session;
  }

  function importSession(release) {
    return new Promise((resolve, reject) => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = ".json,.yaml,.yml";
      input.onchange = () => {
        const file = input.files && input.files[0];
        if (!file) { resolve(null); return; }
        file.text().then((text) => {
          try {
            const parsed = JSON.parse(text);
            saveSession(release, parsed);
            resolve(parsed);
          } catch (e) { reject(new Error("archivo no es JSON válido")); }
        });
      };
      input.click();
    });
  }

  function addEvidence(release, scenarioId, evidence) {
    const session = loadSession(release) || { release, results: [] };
    let entry = (session.results || []).find(r => r.scenario_id === scenarioId);
    if (!entry) {
      entry = { scenario_id: scenarioId, status: "PASS", evidence: [] };
      (session.results || (session.results = [])).push(entry);
    }
    if (!entry.evidence) entry.evidence = [];
    entry.evidence.push(evidence);
    saveSession(release, session);
  }

  function pasteScreenshot(release, scenarioId, callback) {
    document.addEventListener("paste", function handler(ev) {
      document.removeEventListener("paste", handler);
      const items = ev.clipboardData && ev.clipboardData.items;
      if (!items) return;
      for (const item of items) {
        if (item.type && item.type.startsWith("image/")) {
          const blob = item.getAsFile();
          const reader = new FileReader();
          reader.onload = () => {
            addEvidence(release, scenarioId, {
              kind: "screenshot",
              ref: "sha256:clipboard-" + Date.now(),
              note: "pegado desde portapapeles",
            });
            callback(reader.result);
          };
          reader.readAsDataURL(blob);
          return;
        }
      }
    });
  }

  return {
    loadSession, saveSession, exportSession, importSession, addEvidence,
    pasteScreenshot, buildUatSession, fromLegacy, finalizeAndExport,
    nowRfc3339, uuid,
  };
})();