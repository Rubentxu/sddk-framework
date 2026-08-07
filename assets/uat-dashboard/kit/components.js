/* Renderers (vanilla): matrix table, traceability rollup, status pills. */

const UatRender = (() => {
  function pill(status) {
    const cls = String(status || "PENDING").toLowerCase();
    return `<span class="status-pill ${cls}">${status || "PENDING"}</span>`;
  }

  function matrix(plan, session) {
    const rows = [];
    for (const feature of plan.features || []) {
      for (const sc of feature.scenarios || []) {
        const st = session && session.scenario_results
          ? (session.scenario_results[sc.id] || {}).status || "PENDING"
          : "PENDING";
        rows.push([sc.id, feature.name, sc.title, sc.priority, sc.assignee || "developer", pill(st)]);
      }
    }
    const table = document.createElement("table");
    table.innerHTML =
      "<thead><tr><th>ID</th><th>Feature</th><th>Escenario</th><th>Prioridad</th><th>Assignee</th><th>Estado</th></tr></thead>" +
      "<tbody>" + rows.map(r => `<tr>${r.map(c => `<td>${c}</td>`).join("")}</tr>`).join("") + "</tbody>";
    return table;
  }

  function traceability(plan, session) {
    const blocks = [];
    for (const feature of plan.features || []) {
      let covered = 0;
      const scRows = (feature.scenarios || []).map(sc => {
        const st = session && session.scenario_results
          ? (session.scenario_results[sc.id] || {}).status || "PENDING"
          : "PENDING";
        if (st !== "PENDING") covered++;
        return `<tr><td>${sc.id}</td><td>${sc.title}</td><td>${pill(st)}</td></tr>`;
      }).join("");
      const total = (feature.scenarios || []).length;
      const pct = total ? Math.round(100 * covered / total) : 0;
      blocks.push(
        `<h3>${feature.id} — ${feature.name} ${feature.requirement_ref ? `<span class="sub">(${feature.requirement_ref})</span>` : ""}</h3>` +
        `<p class="sub">coverage: ${pct}% (${covered}/${total})</p>` +
        `<table><thead><tr><th>ID</th><th>Escenario</th><th>Estado</th></tr></thead><tbody>${scRows}</tbody></table>`
      );
    }
    return blocks.join("");
  }

  function kpis(plan) {
    let scenarios = 0, p0 = 0;
    for (const feature of plan.features || []) {
      for (const sc of feature.scenarios || []) {
        scenarios++;
        if (sc.priority === "P0") p0++;
      }
    }
    return `
      <div class="kpi"><div class="v">${scenarios}</div><div class="l">escenarios</div></div>
      <div class="kpi"><div class="v">${p0}</div><div class="l">P0</div></div>
      <div class="kpi"><div class="v">${(plan.features || []).length}</div><div class="l">features</div></div>
    `;
  }

  return { pill, matrix, traceability, kpis };
})();
