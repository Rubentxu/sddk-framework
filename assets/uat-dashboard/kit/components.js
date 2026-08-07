/* Renderers (vanilla): matrix table, traceability rollup, status pills,
 * and the v2 wizard primitives (context bar, pre-flight, typed step,
 * typed evidence, failure protocol, teardown).
 */

const UatRender = (() => {
  function pill(status) {
    const cls = String(status || "PENDING").toLowerCase();
    return `<span class="status-pill ${cls}">${status || "PENDING"}</span>`;
  }

  function escapeHtml(s) {
    return String(s == null ? "" : s)
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
  }

  function statusOf(session, scenarioId) {
    if (!session) return "PENDING";
    if (Array.isArray(session.results)) {
      const r = session.results.find(r => r && r.scenario_id === scenarioId);
      if (r && r.status) return r.status;
    }
    if (session.scenario_results && session.scenario_results[scenarioId]) {
      return session.scenario_results[scenarioId].status || "PENDING";
    }
    return "PENDING";
  }

  function matrix(plan, session) {
    const rows = [];
    let i = 0;
    for (const feature of plan.features || []) {
      for (const sc of feature.scenarios || []) {
        rows.push(
          `<tr style="--i:${i++}">` +
            `<td class="id">${escapeHtml(sc.id)}</td>` +
            `<td>${escapeHtml(feature.name)}</td>` +
            `<td>${escapeHtml(sc.title)}</td>` +
            `<td>${(sc.priority || "").toUpperCase()}</td>` +
            `<td>${escapeHtml(sc.assignee || "developer")}</td>` +
            `<td>${pill(statusOf(session, sc.id))}</td>` +
          `</tr>`
        );
      }
    }
    const table = document.createElement("table");
    table.className = "data-table";
    table.innerHTML =
      "<thead><tr><th class=\"id\">ID</th><th>Feature</th><th>Escenario</th><th>Prioridad</th><th>Assignee</th><th>Estado</th></tr></thead>" +
      "<tbody>" + rows.join("") + "</tbody>";
    return table;
  }

  function traceability(plan, session) {
    const blocks = [];
    let featureIdx = 0;
    for (const feature of plan.features || []) {
      const total = (feature.scenarios || []).length;
      let covered = 0;
      const scRows = (feature.scenarios || []).map(sc => {
        const st = statusOf(session, sc.id);
        if (st !== "PENDING") covered++;
        return `<tr><td class="id">${escapeHtml(sc.id)}</td><td>${escapeHtml(sc.title)}</td><td>${pill(st)}</td></tr>`;
      }).join("");
      const pct = total ? Math.round((100 * covered) / total) : 0;
      blocks.push(
        `<section class="trace-feature" style="--i:${featureIdx++}">` +
          `<header style="display:flex;align-items:baseline;justify-content:space-between;gap:var(--space-3);margin-bottom:var(--space-3)">` +
            `<h3 style="font-size:var(--text-md)">${escapeHtml(feature.id)} — ${escapeHtml(feature.name)}` +
              (feature.requirement_ref ? ` <span style="color:var(--text-dim);font-family:var(--font-mono);font-weight:400;font-size:var(--text-sm)">(${escapeHtml(feature.requirement_ref)})</span>` : "") +
            `</h3>` +
            `<span class="progress-label">coverage ${pct}% <span style="color:var(--text-dim)">(${covered}/${total})</span></span>` +
          `</header>` +
          `<table class="data-table"><thead><tr><th class="id\">ID</th><th>Escenario</th><th>Estado</th></tr></thead><tbody>${scRows}</tbody></table>` +
        `</section>`
      );
    }
    if (blocks.length === 0) return `<p class="page-sub">Plan vacío: no hay features para mostrar.</p>`;
    return `<div style="display:grid;gap:var(--space-5)">${blocks.join("")}</div>`;
  }

  function kpis(plan) {
    let scenarios = 0, p0 = 0;
    for (const feature of plan.features || []) {
      for (const sc of feature.scenarios || []) {
        scenarios++;
        if (sc.priority === "P0") p0++;
      }
    }
    return [
      { v: scenarios, l: "escenarios" },
      { v: p0, l: "P0" },
      { v: (plan.features || []).length, l: "features" },
    ].map(k => `<div class="kpi"><div class="v">${k.v}</div><div class="l">${k.l}</div></div>`).join("");
  }

  function userStoryBanner(scenario) {
    const story = scenario.context && scenario.context.user_story;
    if (!story || !story.trim()) return "";
    return `<div class="user-story-banner" style="--i:0"><span class="user-story-label">Intención</span><p>${escapeHtml(story)}</p></div>`;
  }

  function preflightChecklist(scenario) {
    const items = (scenario.context && scenario.context.preconditions) || [];
    if (!items.length) return "";
    const rows = items.map((p, i) =>
      `<li class="preflight-item" style="--i:${i}"><label class="preflight-check"><input type="checkbox" class="preflight-cb" data-preflight="${escapeHtml(p)}"><span>${escapeHtml(p)}</span></label><button class="copy-btn" data-copy="${escapeHtml(p)}">copiar</button></li>`
    ).join("");
    return `<section class="preflight" style="--i:1"><header class="preflight-head"><h3 class="preflight-title">Pre-flight</h3><span class="preflight-hint">Marca cada requisito antes de empezar</span></header><ul class="preflight-list">${rows}</ul></section>`;
  }

  function contextBar(scenario, feature) {
    const timing = scenario.context && scenario.context.timing;
    const risk = scenario.risk;
    const help = scenario.context && scenario.context.help;
    const window = (timing && timing.window) || (scenario.flags && scenario.flags.includes("smoke") ? "smoke" : "regression");
    const est = scenario.est_minutes || 0;
    const ceiling = (timing && timing.timeout_min) || Math.max(est * 2, 5);
    const riskCls = risk ? `risk-${(risk.classification || "medium").toLowerCase()}` : "";
    const riskTxt = risk ? (risk.classification || "medium") : "—";
    const helpParts = [];
    if (help) {
      if (help.slack && help.slack.length) helpParts.push(`<span class="ctx-help-item">${escapeHtml(help.slack.join(" · "))}</span>`);
      if (help.contacts && help.contacts.length) helpParts.push(`<span class="ctx-help-item">${escapeHtml(help.contacts.join(" · "))}</span>`);
      if (help.related_adrs && help.related_adrs.length) helpParts.push(`<span class="ctx-help-item">${escapeHtml(help.related_adrs.join(" · "))}</span>`);
      if (help.docs && help.docs.length) helpParts.push(`<a class="ctx-help-item ctx-help-link" href="#">${escapeHtml(help.docs[0])}</a>`);
    }
    if (feature && feature.requirement_ref) {
      helpParts.push(`<span class="ctx-help-item"><code>${escapeHtml(feature.requirement_ref)}</code></span>`);
    }
    return `<div class="context-bar ${riskCls}" style="--i:0"><div class="ctx-pair"><span class="ctx-label">window</span><span class="ctx-val">${escapeHtml(window)}</span></div><div class="ctx-pair"><span class="ctx-label">est / ceiling</span><span class="ctx-val">${est}m / ${ceiling}m</span></div><div class="ctx-pair"><span class="ctx-label">risk</span><span class="ctx-val">${escapeHtml(riskTxt)}</span></div><div class="ctx-pair ctx-help">${helpParts.join("") || "<span class='ctx-help-item'>—</span>"}</div></div>`;
  }

  function stepBlock(step, i) {
    const kind = (step.kind || "shell").toLowerCase();
    const action = escapeHtml(step.action || "");
    const expected = escapeHtml(step.expected || "");
    const number = step.step || (i + 1);
    const copyHint = step.copy_hint !== false && (kind === "shell" || kind === "api" || kind === "file");
    const header = `<div class="step-head"><span class="step-num">${number}</span><span class="step-kind step-kind-${kind}">${escapeHtml(kind)}</span>${copyHint ? `<button class="copy-btn" data-copy="${escapeHtml(step.action || "")}">copiar</button>` : ""}</div>`;
    let body;
    if (kind === "shell" || kind === "api") {
      body = `<pre class="step-code">${escapeHtml(step.action || "")}</pre>`;
    } else {
      body = `<p class="step-prose">${escapeHtml(step.action || "")}</p>`;
    }
    return `<li class="scenario-step step-${kind}" style="--i:${i}">${header}${body}<p class="step-expected"><strong>Esperado</strong>${expected}</p></li>`;
  }

  function evidenceChips(evidence) {
    if (!evidence || !evidence.length) return "";
    return evidence.map((e, i) => {
      const kind = (e.kind || "note").toLowerCase();
      const ref = e.ref ? escapeHtml(e.ref.slice(0, 24)) + (e.ref.length > 24 ? "…" : "") : "—";
      const size = e.size_bytes ? ` · ${e.size_bytes}B` : "";
      return `<li class="evidence-chip evidence-chip-${kind}" data-evidence-index="${i}"><button class="evidence-remove" data-remove="${i}" title="Eliminar">×</button><span class="evidence-kind">${escapeHtml(kind)}</span><code class="evidence-ref">${ref}</code>${size ? `<span class="evidence-size">${size}</span>` : ""}${e.observed_value != null ? `<span class="evidence-obs">obs=<code>${escapeHtml(e.observed_value)}</code></span>` : ""}${e.note ? `<span class="evidence-note">${escapeHtml(e.note)}</span>` : ""}</li>`;
    }).join("");
  }

  function evidenceCaptureUI(scenario, current) {
    const kinds = (scenario.evidence && scenario.evidence.kinds) || [];
    const fallback = scenario.evidence_prompt
      ? [{ kind: "note", note: scenario.evidence_prompt }]
      : [{ kind: "screenshot" }, { kind: "note" }];
    const list = kinds.length ? kinds : fallback;
    const prompt = scenario.evidence_prompt
      ? escapeHtml(scenario.evidence_prompt)
      : "Captura evidencia para soportar tu verdict";
    const inputs = list.map((k, i) => {
      const kind = (k.kind || "note").toLowerCase();
      if (kind === "screenshot") {
        return `<div class="evidence-input evidence-input-screenshot" style="--i:${i}"><label class="evidence-input-label">📷 Screenshot</label><div class="evidence-drop" id="drop">Pega screenshot aquí (Ctrl+V)</div><button class="btn btn-tiny" data-attach-file="screenshot">Adjuntar archivo</button></div>`;
      }
      if (kind === "file") {
        return `<div class="evidence-input evidence-input-file" style="--i:${i}"><label class="evidence-input-label">📄 Fichero${k.ref ? ` <span class="evidence-input-ref">ref esperado: <code>${escapeHtml(k.ref)}</code></span>` : ""}</label><button class="btn btn-tiny" data-attach-file="file">Adjuntar archivo</button></div>`;
      }
      if (kind === "command_output") {
        return `<div class="evidence-input evidence-input-command" style="--i:${i}"><label class="evidence-input-label">⌨️ Command output${k.ref ? ` <span class="evidence-input-ref">ref: <code>${escapeHtml(k.ref)}</code></span>` : ""}</label><textarea class="evidence-text" rows="3" data-evidence-kind="command_output" placeholder="Pega aquí la salida del comando (stdout+stderr)"></textarea><button class="btn btn-tiny" data-attach-text="command_output">Capturar</button></div>`;
      }
      if (kind === "assertion") {
        return `<div class="evidence-input evidence-input-assertion" style="--i:${i}"><label class="evidence-input-label">✅ Assertion${k.expected_value != null ? ` <span class="evidence-input-ref">expected: <code>${escapeHtml(k.expected_value)}</code> (${escapeHtml(k.match_mode || "exact_match")})</span>` : ""}</label><input class="evidence-text evidence-text-assertion" type="text" data-evidence-kind="assertion" placeholder="observed value"><button class="btn btn-tiny" data-attach-text="assertion">Comparar</button></div>`;
      }
      if (kind === "metric") {
        return `<div class="evidence-input evidence-input-metric" style="--i:${i}"><label class="evidence-input-label">📊 Metric${k.expected_value != null ? ` <span class="evidence-input-ref">expected: <code>${escapeHtml(k.expected_value)}</code></span>` : ""}</label><input class="evidence-text evidence-text-metric" type="text" data-evidence-kind="metric" placeholder="valor"><button class="btn btn-tiny" data-attach-text="metric">Capturar</button></div>`;
      }
      return `<div class="evidence-input evidence-input-note" style="--i:${i}"><label class="evidence-input-label">📝 Nota</label><textarea class="evidence-text" rows="2" data-evidence-kind="note" placeholder="${escapeHtml(k.note || "Observación libre")}"></textarea><button class="btn btn-tiny" data-attach-text="note">Capturar</button></div>`;
    }).join("");
    return `<div class="evidence-section" style="--i:6"><p class="evidence-prompt"><strong>Evidencia</strong>${prompt}</p><div class="evidence-inputs">${inputs}</div><ul class="evidence-list">${evidenceChips(current.evidence)}</ul></div>`;
  }

  function failureProtocolPanel(scenario, observed) {
    const protocol = scenario.context && scenario.context.failure_protocol;
    if (!protocol) return "";
    const onFail = protocol.on_fail || [];
    const checklist = onFail.map((s, i) =>
      `<li class="failure-item" style="--i:${i}"><label class="failure-check"><input type="checkbox" data-failure-check="${escapeHtml(s)}"><span>${escapeHtml(s)}</span></label></li>`
    ).join("");
    return `<section class="failure-panel" id="failure-panel" style="--i:10"><header class="failure-head"><h3 class="failure-title">Failure protocol</h3><span class="failure-hint">Sigue el checklist antes de reportar</span></header><ul class="failure-list">${checklist}</ul><div class="failure-actions"><textarea class="evidence-text failure-observed" rows="2" placeholder="Observado (pegar aquí)">${escapeHtml(observed || "")}</textarea><button class="btn btn-failure" id="copy-defect-report">📋 Copiar defect report</button><input class="evidence-text failure-defect-id" type="text" placeholder="DEF-123 (issue tracker id)" id="defect-id"></div></section>`;
  }

  function teardownChecklist(scenario) {
    const items = (scenario.context && scenario.context.postconditions) || [];
    if (!items.length) return "";
    const rows = items.map((p, i) =>
      `<li class="teardown-item" style="--i:${i}"><label class="teardown-check"><input type="checkbox" class="teardown-cb" data-teardown="${escapeHtml(p)}"><span>${escapeHtml(p)}</span></label></li>`
    ).join("");
    return `<section class="teardown" style="--i:0"><header class="teardown-head"><h3 class="teardown-title">Teardown</h3><span class="teardown-hint">Cleanup tras el scenario</span></header><ul class="teardown-list">${rows}</ul></section>`;
  }

  return {
    pill, matrix, traceability, kpis,
    userStoryBanner, preflightChecklist, contextBar, stepBlock,
    evidenceChips, evidenceCaptureUI, failureProtocolPanel, teardownChecklist,
    escapeHtml,
  };
})();