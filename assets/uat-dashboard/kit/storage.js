/* Storage helpers: localStorage persistence + export/import JSON (zero backend). */

const UAT = (() => {
  const KEY = (release) => `sddk-uat-${release}`;

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

  function exportSession(release) {
    const session = loadSession(release);
    if (!session) return null;
    const blob = new Blob([JSON.stringify(session, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `uat-session-${release}.json`;
    a.click();
    URL.revokeObjectURL(url);
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
    const session = loadSession(release) || { scenario_results: {} };
    if (!session.scenario_results) session.scenario_results = {};
    const entry = session.scenario_results[scenarioId] || { evidence: [] };
    if (!entry.evidence) entry.evidence = [];
    entry.evidence.push(evidence);
    session.scenario_results[scenarioId] = entry;
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
              note: "pegado desde portapapeles"
            });
            callback(reader.result);
          };
          reader.readAsDataURL(blob);
          return;
        }
      }
    });
  }

  return { loadSession, saveSession, exportSession, importSession, addEvidence, pasteScreenshot };
})();
