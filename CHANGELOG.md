# Changelog

All notable changes to this project are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.9.13] - 2026-08-14

Corrige la integridad del bundle de release: el manifest se genera desde rutas
tracked publicables, falla cerrado ante errores Git y rutas no UTF-8, conserva
rutas UTF-8 especiales y se verifica en staging antes de actualizar el runtime.

### Fixes
  - fix(manifest): enumera `git ls-files` limitado a superficies publicables y hashea bytes actuales
  - fix(manifest): drena salida Git concurrentemente y rechaza rutas tracked publicables no UTF-8
  - fix(manifest): serializa rutas UTF-8 especiales con escape reversible
  - fix(dev): verifica bundles descargados en staging antes de tocar el destino
  - fix(release): empaqueta el manifest canónico comprometido y elimina cuatro rutas phantom

## [1.9.12]

Cierra el ciclo SDDK2-009 (phase.build.complete). Cinco work units resueltas: U1+U2 bundle seam (dev install --source + skill-registry writer), U3 knowledge pipeline prefight (-with-knowledge --approve con quarantine rule), U4 --outcome passed en todos los evaluate-gate, U5 bump 1.9.12 + BACKLOG + CHANGELOG.

### Features
  - feat(dev): add --source flag to dev install — copies MANIFEST and verifies SHA256
  - feat(dev): add --write-registry to dev link — writes skill-registry.md to XDG project dir
  - feat(dev): write_skill_registry() — scans skills/*/SKILL.md, skips _shared, writes sorted markdown table
  - feat(agent): init knowledge preflight with --with-knowledge --approve pipeline
  - feat(cli): add --outcome passed to all evaluate-gate calls (8 SKILL.md files)

### Fixes
  - fix(cli): --outcome passed added to evaluate-gate in all phase SKILLs
  - fix(backlog): SDDK2-009 inserted after SDDK2-008.DEBT

## [1.9.11]

Cierra el ciclo SDDK2-008 (phase0-knowledge-ingestion). El pipeline `scan → plan → import → verify` con CAS, provenance, authority y quarantine está gobernado por la knowledge vault en `~/.sddk-knowledge/`. Tres negative tests aseguran que `--approve` en candidatos Quarantine (R10) o con razón "relation conflicts" (R5 surface) son rechazados por `is_approvable_change()`. Distribución corregida: 7 crates en `version.workspace = true` alineados a 1.9.11.

### Features
  - feat(knowledge): scan → plan kp-<hex16> → import → verify governed pipeline
  - feat(knowledge): TOCTOU cerrada por re-hash en import
  - feat(knowledge): Authority::Trusted exige disposition=Import o --approve + is_approvable_change
  - feat(knowledge): receipt kr-<hex16> determinista para not_applicable
  - feat(knowledge): CliFixture + git_commit_all scaffolding para integración tests

### Fixes
  - fix(dist): 7 crates con version.workspace=true alineados a 1.9.11

### Other
  - test(knowledge): approve_quarantine_candidate_fails — R10 negative test
  - test(knowledge): approve_relation_conflict_candidate_fails — R5 surface negative test
  - test(knowledge): relation_key_is_deterministic_for_path_invariants — case normalization invariant
  - docs(BACKLOG): SDDK2-008 y SDDK2-008.DEBT insertados entre SDDK2-007 y SDDK2-101

## [1.9.10] - 2026-08-14

Cierra el release del ciclo SDDK2-006 (`sddk-2-0-phase0-doc-governance`). Tras el bump inicial a `v1.9.9` y el commit `docs(handoff): refresh with final HEAD dbf93c7` (`cbe26db`) que reescribió el handoff con el SHA final, el tag `v1.9.9` quedó apuntando al commit previo (`dbf93c7`) sin cubrir el refresh de handoff. Se corta `v1.9.10` como tag anotado en un nuevo `chore(release)` para preservar la linear-history (AGENTS.md §2.2) y satisfacer el contrato `sddk-release` que exige tag-peels-to-HEAD. Sin cambios de código de producción; el diff acumulado del ciclo sigue siendo puramente documental (SDDK2-006 fue zero-intrusion por diseño).

### Other
  - chore(release): bump to v1.9.10 (sddk-2-0-phase0-doc-governance) — corte de tag post-handoff-refresh; repara tag/HEAD gap documentado como W2 en el vault

## [1.9.9] - 2026-08-13

Split AGENTS.md into stable/history/handoff surfaces (SDDK2-006 doc-governance).

### Other
  - feat(docs): split AGENTS.md — stable ≤150 LOC + history archived + handoff; renumber BACKLOG SDDK2-004→006 / SDDK2-005→007; reconcile vault ID collision

## [1.9.1] - 2026-08-11

Cosmetic fixes and documentation improvements from post-release cleanup.

### Other
  - chore(docs): bootstrap.sh — rename `SHARED_DIR` → `SDDK_FRAMEWORK_ROOT` for clarity; the variable always pointed to the CWD but the name was misleading
  - docs(sdk): add "resolved state" section to SPEC.md documenting the 2026-08-08 elimination of `~/.sddk-shared/` and current verified state

## [1.9.0] - 2026-08-11

Guided Runner UX (F13, M-002): a human-governed UAT flow with immutable sign-off, stale advisories, blind checks, evidence gates, checkpoints, diagnostics, and designer/runner/reviewer modes. Minor bump for the new RF-024..028 capabilities, 13 domain types, and plan schema v4.

### Features
  - feat(uat): F13 Guided Runner UX — immutable SHA-256 sign-off, stale advisory, blind checks with evidence gate, checkpoints with AI diagnostics, and designer/runner/reviewer modes
  - feat(uat): RELEASE ACCEPTANCE wizard with immutable acceptance records and release gate integration for RF-024..028
  - feat(domain): 13 UAT domain types for runner modes, blind checks, completion policies, checkpoints, diagnostics, acceptance, and staleness
  - feat(uat): plan schema v4 with backwards-compatible parsing of schema v3

## [1.8.1] - 2026-08-11

Endurece el CI local (act + podman): el lint de `dev-doc-check` ya enforza SDK009/SDK010 para los docs/inventory regenerados, así que los steps redundantes `generate docs/inventory --check` en `ci.yml` se eliminan (bajo `act` con bind mount el check directo daba falsos "stale"). Patch bump por fix + chore (sin features nuevas); el CI local queda verde con un solo gate lint.

### Fixes
  - fix(ci): eliminar steps redundantes de `generate docs/inventory --check` — el lint `dev-doc-check` ya valida SDK009/SDK010 (sha256-pinned entries, INVENTORY sync) como gate único de los docs/inventory regenerados; bajo `act` con bind mount el check directo daba falso stale y hacía fallar el workflow aunque el contenido estuviera sincronizado

### Other
  - chore(style): `cargo fmt --all` en workspace — uniforma el estilo de los 7 crates; 72 diffs (20 del ciclo surface-brevity + 52 pre-existentes de v1.7.0); CI local (act) verde

## [1.8.0] - 2026-08-11

Cierra la deuda INC-001 (surface-brevity-standard) y formaliza el estándar de concisión de superficies (ADR-016). El orquestrador pasa de 1366 líneas a un shell de 288 que delega MCW/políticas/tablas a `prompts/sddk/`; el doctor detecta superficies que exceden el umbral y subdirectorios vacíos. Minor bump por dos features (`feat(dev)` ×2) más un refactor estructural.

### Features
  - feat(dev): `sddk dev doctor` surface.briefness — detecta agentes/skills/prompts que exceden el umbral (300/150/200 líneas); `--strict` promueve la violación a exit 1; por defecto es advisory en el report
  - feat(dev): `sddk dev doctor` surface.empty_dirs — detecta subdirectorios vacíos o phantom en agents/skills/prompts; se mantiene advisory bajo `--strict` (no auto-elimina); elimina la skill fantasma `skills/logseq-vault/`

### Refactors
  - refactor(agents): `agents/orchestrator.md` shell ≤300 — extrae arsenal, dynamic-workflow, escalation-policy, status-query, entropy-policy y document-catalog a `prompts/sddk/`; routing A–D, gates y comandos preservados; tabla MCW step index retirada del shell
  - docs(adr): ADR-016 surface-brevity — agentes ≤300 / skills ≤150 / prompts ≤200 líneas; estructura Pocock (frontmatter + workflow + examples); sin excepciones nominales; `sddk dev doctor` lo enforza como advisory, `--strict` lo promueve

### Other
  - chore(agents): prune `skills/logseq-vault/` (skill fantasma, directorio vacío preexistente; el doctor lo detectaba pero no lo eliminaba)
  - chore(agents): `skills/_shared/` se mantiene como referencia técnica no-namespace (no es skill ejecutable; queda fuera del scope doctor)

## [1.6.1] - 2026-08-10

Endurece la release local CI/CD-independent: el workflow SDDK no depende de ningún sistema CI/CD (CI/CD queda como distribución opcional posterior al tag), con reconciliación idempotente de receipts, precondiciones de trunk/HEAD/cycle y autorización efectiva de `git.inspect`. Patch bump por refactor + fix (sin features nuevas).

### Fixes
  - fix(release): endurecer release local CI/CD-independent — recibos `git.push`/`git.tag` `Started` reconciliados contra el efecto remoto por SHA (los pre-efecto se reintentan, los post-efecto cierran sin duplicar), ciclo ligado a trunk/HEAD (exige trunk limpio y `HEAD` ancestro del commit del manifest), `--cycle` propagado por CLI/agente/skill/prompt, `git.inspect` añadido a la autorización efectiva, orden release → archive coherente y prohibición de comandos ejecutables PR/CI/CD del proveedor

### Other
  - refactor(release): desacoplar workflow SDDK de CI/CD — ruta de release local `validate → push main → verificar SHA remoto → tag anotado` idempotente; Forge integración opcional, nunca gate ni autoridad; precondiciones locales exigen trunk limpio y `HEAD` ancestro del commit del manifest

## [1.6.0] - 2026-08-10

Consolida la integridad UAT fail-closed (P0) y el vault persistente por identidad estable (P1), cierra el loop dashboard → control plane (wizard → ingest), normaliza las superficies a `sddk-*` con cero intrusión (ADR-0011) y elimina el segundo checkout `~/.sddk-shared/` a favor del modelo asdf-vm (CWD + bundle XDG). Minor bump por las dos features (`feat(uat)` + `feat(persistence)`).

### Features
  - feat(uat): integridad UAT fail-closed con gate de release (P0) — el gate `release-uat-approved` exige sesión humana con verdict y verifica build fingerprint (commit/branch/tag/dirty) antes de permitir el tag; `sddk uat gate release --tag X` emite `BLOCKED`/`ALLOWED` con recovery plan cuando hay mismatch
  - feat(persistence): vault por identidad estable con CLI knowledge (P1) — `sddk vault <id>` resuelve el vault XDG del proyecto por identidad (no por path), `sddk knowledge` añade listado/búsqueda/export del vault (markdown + JSON)
  - feat(uat): schema v2 — plan con `context.{user_story, preconditions, workspace, timing, help, failure_protocol, postconditions, test_data}`, session con `metadata.{tester, env_fingerprint, build, duration_ms}`, evidence tipada (`file | screenshot | command_output | assertion | metric | note`), risk + automation + provenance, manifest XDG-resident con sha256-pinned entries + `sddk uat verify-integrity` (exit 0=ok / 0=partial / 1=fail)
  - feat(uat): history aggregator — `sddk uat history --release X --plan P --sessions S1 [--sessions S2 ...]` con per-scenario `runs_total/passing/failing/blocked`, `success_rate`, `flakiness_score`, `first/last_run` (con commit + tester_id), `defect_ids[]`, `avg/p95_duration_ms`, `trend`
  - feat(uat): wizard v2 (browser) — pre-flight checklist, sticky context bar (window/est-ceiling/risk/help), typed steps (shell/api → `<pre>`, ui/file/manual → prose), typed evidence capture por `evidence.kinds[]`, failure protocol flow con checklist + auto-filled defect template + clipboard copy + `linked_defect`, teardown checklist, persistent tester id `T-XXXX`
  - feat(uat): wired dashboard → control plane — `sddk uat open` levanta HTTP server en `127.0.0.1:0` (OS-assigned), wizard POSTea `/ingest`, server cierra con Ctrl+C vía `AtomicBool` shutdown flag. Mismo origen (GET / sirve el wizard HTML) → sin CORS
  - feat(uat): suggester + apply — `sddk uat scenario-context --plan FILE [--apply]` reglas deterministas (timing desde `est_minutes`, preconditions desde `step.kind`, risk desde `priority`, evidence default Note, automation Manual, provenance desde plan metadata); `user_story` queda placeholder para humano/LLM

### Fixes
  - fix(uat): wizard script order — `storage.js` debe cargar antes de `plan.js`/`wizard.js` (window.storage undefined rompía init)
  - fix(uat): collapse nested if-let en `apply_suggestion` user_story branch (clippy collapsible_if)
  - fix(uat): `uat history` acepta `--sessions X Y` (positional, `num_args = 1..`) además de `--sessions X --sessions Y`
  - fix(docs): replace all `.sddk-shared/` paths con CWD + XDG bundle runtime — 12 referencias en 8 archivos (AGENTS.md, docs/, scripts/, knowledge vault)

### Other
  - refactor(namespace): normalizar superficies a `sddk-*` y cero intrusión (ADR-0011) — `orchestrator`/`sddk-*`/`prompts/sddk/` activos; aliases `sdd-*`/`sdd-kernel-*`/`gentle-orchestrator` eliminados; cero ficheros framework plantados en repos de proyectos
  - docs(agents): AGENTS.md — directorio layout (asdf-vm inspired) + regresiones detectadas + recovery procedures + pre-commit checklist + 3 roles (repo de desarrollo / bundle runtime / workspace de uso) + resolution order
  - docs(agents): add session handoff section (current state + next steps) — qué está implementado, qué queda pendiente, cómo reabrir la sesión
  - docs(generated): regenerar inventory/workflow y alinear SPEC/BACKLOG/ADR con cero intrusión — alineado con RS-2026-08 / CP-2026-08

## [1.5.3] - 2026-08-07

Cierra U5 del milestone UAT-2026-08: el gate `release-uat-approved` deja de ser inerte — ahora se evalúa contra la config del proyecto (XDG) por tipo de release.

### Features
  - feat(uat): `sddk uat config show|set` — config per-proyecto XDG-resident (`~/.local/share/sddk/projects/<id>/uat.toml`): política `release_gate` por tipo (major/minor/patch → required/skip/advisory), `human` (developer/architect availability), `activation` (umbrales min_features/min_diff_lines/critical_domains). Default: major+minor=required, patch=skip.
  - feat(uat): `sddk uat gate release --tag X [--previous-tag Y|--release-type major|minor|patch]` — evalúa `release-uat-approved` para el release type derivado (semver diff). Emite `BLOCKED` con plan de recovery cuando `required`, `ALLOWED` cuando `skip`/`advisory`. JSON para orquestadores.
  - feat(uat): `UatConfig` + `ReleaseGateAction` (required/skip/advisory) + `ReleaseType` (major/minor/patch) en `sddk-domain`. Funciones puras: `evaluate_release_gate()`, `release_type_from_diff()`.

### Fixes
  - fix(uat): gate `release-uat-approved` ya no es inerte — antes declarado sin requires en transiciones; ahora evaluado dinámicamente por el orchestrator antes de tagear.

## [1.5.2] - 2026-08-07

Consolida el milestone UAT-2026-08 (U1-U7) y las correcciones post-1.5.0: cierra el loop humano end-to-end (wizard canónico → ingest → failures → agente estudia).

### Features
  - feat(uat): `sddk uat plan/validate/dashboard/ingest/report/status` — data-driven YAML canónico (ADR-0012)
  - feat(uat): `sddk uat open` — render dashboard + abrir en navegador del sistema sin servidor (file://); SO-aware (xdg-open/open/cmd-start); `--browser` override
  - feat(uat): `sddk uat failures` — lista FAIL/BLOCKED con contexto completo (feature, priority, assignee, rationale, comment, evidence); JSON para que el agente estudie cada fallo
  - feat(uat): dashboard kit en bundle (`assets/uat-dashboard/`) — kit/templates/views (guided/matrix/traceability); templates HTML inlinean JS+CSS (100% autocontenido, ADR-0010)
  - feat(uat): workflow fase `uat` + status `UAT_WAITING` + gates `uat-activated/uat-verdict/release-uat-approved` (ADR-0012)
  - feat(uat): control plane `uat_results` (verdict, coverage, defects por tag_version) + panel "UAT readiness" en dashboard de telemetría
  - feat(uat): 4 agentes (`uat-planner/guide/runner/reporter`) + 4 skills (`uat-dashboard/traceability/guided-mode/evidence`)

### Fixes
  - fix(uat): views HTML inlinean storage.js/components.js (Chrome bloqueaba scripts file:// vía CORS — el HTML ahora es 100% autocontenido y abre vía file://)
  - fix(uat): wizard canónico — `Finalizar y exportar reporte` genera JSON con la forma exacta de `UatSession` (schema_version, executor, executed_by, started_at, finished_at, results con evidence por hash); compatible directo con `sddk uat ingest`
  - fix(uat): guard de integridad en `uat ingest` — `executor: human` exige `executed_by` + `finished_at` + (evidencia o non-PASS); rechaza sesiones humanas fabricadas
  - fix(agents): `uat-planner` craft rule 9 — quoting YAML-safe (textos con `:` rompen el plan; hallazgo del dogfooding)
  - fix(skills): contradicciones ADR-0011 v3.5 — `adopt apply` ya no planta `workflow/workflow.yaml`; política Local-Only v3.3→v3.5 (docs al knowledge vault)
  - fix(tests): workspace completo verde 202+ tests (AdoptionStoragePaths new fields en test domain + unused binary)

## [1.4.0] - 2026-08-07

### Features
  - feat(uat): milestone UAT-2026-08 U1-U7 — dashboard kit en bundle (assets/uat-dashboard), dominio uat.rs, CLI uat plan/validate/dashboard/ingest/report/status, workflow fase uat + status UAT_WAITING + gates uat-activated/uat-verdict/release-uat-approved, control plane uat_results + panel "UAT readiness" en dashboard telemetría, agentes uat-planner/guide/runner/reporter + 4 skills (ADR-0012/0013, RF-019/020, RNF-010)
  - feat(uat): U8 dogfooding parcial — uat-plan v1.5.0 (6 features, 13 escenarios), dashboard guiado generado y validado (determinismo, cero URLs externas); la sesión humana queda PENDIENTE de validación real (la sesión inicial fue fabricada por el agente y eliminada del control plane)

### Fixes
  - fix(agents): uat-planner craft rule 9 — quoting YAML-safe (colon-space rompe el plan; hallazgo del dogfooding)
  - fix(skills): contradicciones ADR-0011 — adopt no planta workflow.yaml (C1/C2), política Local-Only v3.3→v3.5 (C3/C4, docs al knowledge vault)
  - fix(tests): workspace completo verde — AdoptionStoragePaths new fields en test domain + unused binary (202 tests PASS)

## [1.4.0] - 2026-08-07

### Features
  - feat(telemetry): G5 research packet cross-proyecto — analytics research --all-projects desde control plane + resumen por proyecto (CP-2026-08)
  - feat(rs): RS-6 resolución de versión asdf — sddk version con .sddk-versions → current → path: (ADR-0011)
  - feat(rs): RS-5 bundle runtime multi-versión — dev use (asdf-style) + dev link/update resuelven framework activo (ADR-0011)
  - feat(rs): RS-4 generate docs/inventory → XDG por defecto con --in-repo explícito (ADR-0011)
  - feat(rs): RS-3 cycle artifacts en XDG — cycle artifacts-dir + prompts/skills a {cycle-artifacts-dir} (ADR-0011)
  - feat(rs): RS-1 multiplataforma dirs + RS-2 adopt/lint no intrusivos — cero ficheros framework en repos de proyectos (ADR-0011)
  - feat(telemetry): control plane local — telemetry ingest/aggregate/status/dashboard + metrics record upsert (CP-2026-08 G1-G4)
  - feat(distribution): ALL Linux builds standalone (musl static) — aarch64 included (#92)
  - feat(validation): E2E suite — install variants, render, multi-language validation (#91)

### Fixes
  - fix(rs): framework_agent_names fallback a agentes del bundle sin permissions.yaml (RS-7 migración)
  - fix(ci): update release PR branch when behind before auto-merge (#83)
  - fix(ci): tag-release reads version from origin/main, not dirty worktree (#86)

### Other
  - docs(control-plane): CP-2026-08 IMPLEMENTADO — README control plane, milestone cerrado, backlog 49/49 (ADR-0009/0010)
  - docs(roadmap): RS-2026-08 IMPLEMENTADO — milestone cerrado, backlog E12 completa (ADR-0011)
  - docs(roadmap): milestones CP-2026-08 (ADRs 0009/0010) + RS-2026-08 (ADR-0011, modelo asdf, multiplataforma) — specs, PRD, backlog
  - docs(control-plane): CP-2026-08 milestone — ADRs 0009/0010, spec, PRD RF-016/017, roadmap, backlog E11
  - docs(validation): N3 editor checklist PASS — E2E-2026-08 fully closed (8/8)
  - docs(roadmap): E2E-2026-08 milestone implemented — 7/7 suites PASS (#91)
  - test(cli): environment-robust doctor test; docs: local-first CI via act (#90)
  - docs(validation): E2E validation plan — install, deploy, multi-language, render (#89)

## [1.3.0] - 2026-08-06

### Features
  - feat(cli): completion install — installs shell completions (#84)

### Fixes
  - fix(ci): gh pr list --head does not glob; filter with startswith (#81)

## [1.2.0] - 2026-08-06

### Features
  - feat(ci): release robot — cron poller that removes all bot friction (#79)

### Fixes
  - fix(ci): dispatch Release workflow explicitly from tag-release (#77)

## [1.1.0] - 2026-08-06

### Features
  - feat(ci): fully automatic release pipeline (#71)
  - feat(distribution): hardened installer, completions, signed assets, brew tap (#66)
  - feat(install): interactive installer with framework release bundle (#64)

### Fixes
  - fix(ci): trigger via Auto-merge workflow_run + anti-loop (#74)
  - fix(ci): extract pending tag from explicit new-tag line (#73)
  - fix(ci): reindent release PR body block (invalid YAML) (#72)
  - fix(install): cosign identity regexp + dev update creates missing root (#70)
  - fix(release): pin cosign v3.8.1 — v4.1 sign-blob breaks on output-signature/certificate (#69)
  - fix(release): build darwin-x86_64 cross from arm64 runner (macos-13 retired) (#68)
  - fix(agents): normalize frontmatter models to provider-qualified names (#63)

### Other
  - docs(validation): v1.0.0 published (#62)

## [1.8.0] - 2026-08-11
