# Backlog técnico — SDDK v3.6

**Estado auditado:** 2026-08-04 (v3.6 completo); 2026-08-07 añadida épica E11 (CP-2026-08, planificada)
**Baseline:** `v0.14.0`
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> El estado mide criterios de aceptación demostrados en el repositorio actual. Código presente sin integración, gate automático o evidencia suficiente se marca como parcial. Nada de este paquete se considera entregado hasta quedar versionado y protegido por CI.

## Resumen de estado

| Estado | Historias | Significado |
| --- | ---: | --- |
| Completa | 49 | Todos los criterios de la historia tienen implementación y prueba directa. |
| Parcial | 0 | No queda ninguna historia parcial. |
| Desviada | 0 | No queda ninguna desviación contractual conocida. |
| No iniciada | 0 | El backlog completo (v3.6 + E11 + E12) está cerrado. |
| **Total** | **49** | v3.6 + CP-2026-08 + RS-2026-08 completos. |

## Matriz de aceptación

| Historia | Estado | Evidencia actual | Gap que impide cerrarla |
| --- | --- | --- | --- |
| SDDK-101 | Completa | `workflow/workflow.yaml`; `validate_workflow`; tests de dominio; `sddk validate schema` con dereferencia offline | Sin gap funcional demostrado. |
| SDDK-102 | Completa | `sddk generate docs`; SDDK009; tests deterministas; gate CI | Sin gap funcional demostrado. |
| SDDK-201 | Completa | SDDK001 y tests de referencias tipadas | Sin gap funcional de historia; falta automatizarla en CI a nivel roadmap. |
| SDDK-202 | Completa | SDDK002-SDDK004 y fixtures de shell ejecutable | Sin gap funcional de historia; el escaneo de fences es opt-in por diseño. |
| SDDK-203 | Completa | `sddk generate inventory`; SDDK010; README enlaza el inventario | Sin gap funcional demostrado. |
| SDDK-301 | Completa | Identidad remote/scope/UUID, receipt persistido y test contractual | Sin gap funcional demostrado. |
| SDDK-302 | Completa | Receipt v2, hash de configuración y rename atómico | Sin gap funcional demostrado. |
| SDDK-303 | Completa | `adopt repair`; tests ReceiptOnly/LedgerOnly/conflicto/corrupción | Sin gap funcional demostrado. |
| SDDK-401 | Completa | SQLite v1, WAL, foreign keys y migración transaccional | La evolución v2+ queda como riesgo del roadmap, no de este criterio inicial. |
| SDDK-402 | Completa | Cadena hash, triggers append-only, `sddk ledger verify` y test de corrupción | Sin gap funcional demostrado. |
| SDDK-403 | Completa | `frame_id` y `command_id` compartidos por comando y `sddk ledger events --frame` | Sin gap funcional demostrado. |
| SDDK-404 | Completa | `sddk cycle rebuild` restaura la base vacía desde eventos sin reescribir el ledger | Sin gap funcional demostrado. |
| SDDK-501 | Completa | Rechazo de transición, source, artifacts, gates y paths | Sin gap funcional de la API de engine; expuesto por CLI vía `cycle transition`. |
| SDDK-502 | Completa | Leases con owner, expiry, fencing token y `cycle lock acquire/release/status`; transición exige fence si hay lease | Sin gap funcional demostrado. |
| SDDK-601 | Completa | Runner tipado (argv separado, env allowlist, timeout y truncado) en `sddk-gateway` | Sin gap funcional demostrado. |
| SDDK-602 | Completa | `ScopedFs` con raíces restringidas, rechazo de escapes/symlinks y escritura atómica | Sin gap funcional demostrado. |
| SDDK-603 | Completa | `GitExecutor` tipado: inspect, create-branch, commit y tag con postcondiciones verificadas contra Git real | Sin gap funcional demostrado. |
| SDDK-604 | Completa | CAS `ArtifactStore` con SHA-256 obligatorio, deduplicación por contenido y verificación en lectura | Sin gap funcional demostrado. |
| SDDK-701 | Completa | Validación JSON Schema runtime con dereferencia local de `$ref` y `sddk validate agent-result` | Sin gap funcional demostrado. |
| SDDK-702 | Completa | Adaptador legacy (`convert_legacy_map`/`convert_legacy_text`) con warnings de campos no verificables y `sddk agent-result convert` | Sin gap funcional demostrado. |
| SDDK-703 | Completa | `permissions.yaml` + `PermissionPolicy` default-deny y gate en `capability apply --agent/--phase` + `sddk permission check` | Sin gap funcional demostrado. |
| SDDK-801 | Completa | Trait `Forge` neutral sin tipos de proveedor, `MockForge` y tests de contrato | Sin gap funcional demostrado. |
| SDDK-802 | Completa | Adaptador `GitHubForge` vía `gh` con runner tipado y tolerancia a ya-mergeado/ya-publicado | La integración contra GitHub real queda como prueba manual; el parseo y postcondiciones están testeados con runner inyectado. |
| SDDK-803 | Completa | `reconcile_pending` finaliza receipts `started` consultando la realidad del proveedor | Sin gap funcional demostrado. |
| SDDK-804 | Completa | `plan_release`/`apply_release` en Rust con secuencia canónica, idempotencia y convergencia tras interrupciones | Sin gap funcional demostrado. |
| SDDK-901 | Completa | Parser de vault: frontmatter, IDs, tipos, títulos, wikilinks y backlinks en `sddk-vault` | Sin gap funcional demostrado. |
| SDDK-902 | Completa | Índice FTS5 reconstruible e incremental por hash (tags, enlaces, backlinks, status) | Sin gap funcional demostrado. |
| SDDK-903 | Completa | Validación VAULT001-VAULT004: ids, títulos y wikilinks rotos | Sin gap funcional demostrado. |
| SDDK-904 | Completa | Grafo `petgraph`: ciclos, camino de muestra y orden topológico | Sin gap funcional demostrado. |
| SDDK-1001 | Completa | `sddk dev doctor|check|install|verify|uninstall` (equivalente a xtask) | Sin gap funcional demostrado. |
| SDDK-1002 | Completa | Receipt `sddk-install.json` con versión, commit, SHA-256, canal y timestamp; verificación y desinstalación atómicas | Sin gap funcional demostrado. |
| SDDK-1003 | Completa | `sddk release dist` genera binario, checksums.txt, sbom.json y attestation.json; `release verify` valida todo | Sin gap funcional demostrado. |
| SDDK-1005 | Completa | Packs declarativos (RF-012/ADR-0004): `manifest.toml`, validación PACK001-007, `sddk pack validate` y SDDK014 | Sin gap funcional demostrado. |
| SDDK-1006 | Completa | Indexación incremental del vault por hash de contenido (RNF-004) y profundidad FTS con tags/enlaces/backlinks (RF-009) | Sin gap funcional demostrado. |
| SDDK-1007 | Completa | Envolvente de error estructurada (RNF-006): código estable, causa y recuperación en errores del runtime | Sin gap funcional demostrado. |
| SDDK-1101 | Completa | Gaps de datos: costos/tokens estimados por modelo, teleological coherence, context quality real, verdict con receipt (RF-016) | Sin gap funcional demostrado. |
| SDDK-1102 | Completa | Store SQLite central `control-plane.sqlite` (projects/cycles/aggregates) reconstruible (RF-016/ADR-0009) | Sin gap funcional demostrado. |
| SDDK-1103 | Completa | `sddk telemetry ingest` cross-proyecto con upsert idempotente y derive desde ledger (RF-016) | Sin gap funcional demostrado. |
| SDDK-1104 | Completa | `sddk telemetry aggregate` cross-proyecto reusando `compute_aggregate` (RF-016) | Sin gap funcional demostrado. |
| SDDK-1105 | Completa | `sddk telemetry dashboard` HTML autocontenido sin CDN (RF-017/ADR-0010) | Sin gap funcional demostrado. |
| SDDK-1106 | Completa | Research packet cross-proyecto para agentes self-research (RF-016, sin MCP) | Sin gap funcional demostrado. |
| SDDK-1201 | Completa | Adopción no intrusiva: eliminar plantado de `workflow/workflow.yaml` en el repo (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1202 | Completa | Artefactos de ciclo en XDG (`cycle-artifacts/{cycle_id}/`) + prompts/skills actualizados (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1203 | Completa | `generate docs/inventory` → XDG por defecto con `--in-repo` explícito (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1204 | Completa | `lint` lee manifest embebido/bundle, no exige `workflow.yaml` en el repo (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1205 | Completa | Bundle runtime `$SDDK_DATA_DIR/framework/<v>/` multi-versión + `dev use` + link → `current` (ADR-0011/asdf) | Sin gap funcional demostrado. |
| SDDK-1206 | Completa | Migración: limpiar receipts duplicados, mover `sddk/` a XDG, re-linkear editores (ADR-0011) | Sin gap funcional demostrado. |
| SDDK-1207 | Completa | Resolución de versión por proyecto: `.sddk-versions` → `current` → `path:` (ADR-0011/asdf) | Sin gap funcional demostrado. |
| SDDK-1208 | Completa | Resolución multiplataforma con crate `dirs`: macOS `~/Library/...`, Windows `%APPDATA%` (ADR-0011) | Sin gap funcional demostrado. |

## ÉPICA E1 — Fuente canónica del workflow

### SDDK-101 Crear `workflow.yaml`

**Prioridad:** P0
**PR:** 1

**Criterios de aceptación:**

- Estados y fases aparecen una sola vez.
- Cada transición declara precondiciones, gates y artefactos.
- Se valida contra schema.

### SDDK-102 Generar documentación del workflow

**Prioridad:** P1
**PR:** 2

- Mermaid y tablas se generan automáticamente.
- CI falla si los generados están obsoletos.

## ÉPICA E2 — Consistencia del repositorio

### SDDK-201 Detectar referencias rotas

**Prioridad:** P0
**PR:** 2

- Detecta agentes, skills, plugins y rutas inexistentes.
- Código de error estable `SDDK001`.

### SDDK-202 Detectar placeholders literales

**Prioridad:** P0
**PR:** 2

- Detecta `{project}`, `~` no expandible en scripts y variables no definidas.

### SDDK-203 Inventario generado

**Prioridad:** P1
**PR:** 1

- El README no mantiene manualmente números de agentes o skills.
- CI falla con `SDDK010` si el inventario generado está obsoleto.

## ÉPICA E3 — Identidad y adopción

### SDDK-301 Resolver identidad lógica

**Prioridad:** P0
**PR:** 3

- Remote normalizado.
- Scope de monorepo.
- Fallback UUID sin remote.

### SDDK-302 Crear receipt de adopción

**Prioridad:** P0
**PR:** 3

- Escritura temporal y rename atómico.
- Incluye versión y hash de configuración.

### SDDK-303 Reparar adopción interrumpida

**Prioridad:** P1
**PR:** 3

- `sddk adopt repair` detecta y resuelve estados parciales.

## ÉPICA E4 — Ledger

### SDDK-401 Crear esquema SQLite

**Prioridad:** P0
**PR:** 4

- WAL y foreign keys activos.
- Migraciones integradas.

### SDDK-402 Implementar cadena hash

**Prioridad:** P0
**PR:** 4

- `sddk ledger verify` detecta alteración o huecos.

### SDDK-403 Implementar frames

**Prioridad:** P1
**PR:** 4

- Todos los eventos de un comando comparten `frame_id`.

### SDDK-404 Replay de estado

**Prioridad:** P0
**PR:** 4

- Reconstruye ciclos desde eventos en una base vacía.

## ÉPICA E5 — Máquina de estados

### SDDK-501 Validar transición

**Prioridad:** P0
**PR:** 4

- Rechaza transición no declarada.
- Explica gates ausentes.

### SDDK-502 Bloqueo y recuperación

**Prioridad:** P0
**PR:** 4

- Locks con owner y lease.
- Recuperación segura de locks huérfanos.

## ÉPICA E6 — Capacidades

### SDDK-601 Runner sin shell arbitrario

**Prioridad:** P0
**PR:** 5

- Programa y argumentos separados.
- Environment allowlist.
- Captura stdout/stderr.

### SDDK-602 Filesystem tipado

**Prioridad:** P0
**PR:** 5

- Escrituras atómicas.
- Paths restringidos al proyecto y vault.

### SDDK-603 Git local

**Prioridad:** P0
**PR:** 5

- Inspect, branch, commit y tag.
- Postcondiciones verificadas con Git real.

### SDDK-604 Almacén de artefactos

**Prioridad:** P1
**PR:** 5

- Deduplicación SHA-256.
- Metadata en SQLite.

## ÉPICA E7 — Agentes

### SDDK-701 Schema de resultados

**Prioridad:** P0
**PR:** 6

- Versionado.
- Artefactos y evidencia tipados.

### SDDK-702 Adaptador legacy

**Prioridad:** P1
**PR:** 6

- Convierte salidas actuales a resultado estructurado.
- Emite warnings y campos no verificables.

### SDDK-703 Permisos por fase

**Prioridad:** P0
**PR:** 6

- Cada agente declara fases y capacidades permitidas.

## ÉPICA E8 — Forge y release

### SDDK-801 Trait `Forge`

**Prioridad:** P0
**PR:** 7

- No contiene tipos específicos de GitHub.

### SDDK-802 Adaptador GitHub

**Prioridad:** P0
**PR:** 7

- Crear PR, leer checks, merge y release.

### SDDK-803 Reconciliación de efectos

**Prioridad:** P0
**PR:** 7

- Resuelve operaciones `unknown` consultando GitHub.

### SDDK-804 Corregir secuencia release

**Prioridad:** P0
**PR:** 1 y 7

- Nunca intenta fusionar después de esperar estado merged.

## ÉPICA E9 — Vault

### SDDK-901 Parser de frontmatter

**Prioridad:** P0
**PR:** 8

- IDs, tipos, relaciones y procedencia.

### SDDK-902 Backlinks e índice FTS

**Prioridad:** P1
**PR:** 8

- Reindexación incremental por hash.

### SDDK-903 Validación de relaciones

**Prioridad:** P0
**PR:** 8

- Relaciones rotas y tipos inválidos producen errores estables.

### SDDK-904 Proyección `petgraph`

**Prioridad:** P2
**PR:** 8

- Ciclos, caminos y orden topológico.

## ÉPICA E10 — Distribución

### SDDK-1001 `xtask install-dev`

**Prioridad:** P1
**PR:** 9

- fmt, clippy, tests, release, install y doctor.

### SDDK-1002 Receipts de instalación

**Prioridad:** P1
**PR:** 9

- Versión, commit, hash y canal.

### SDDK-1003 Publicación estable

**Prioridad:** P1
**PR:** 9

- Binarios, checksums, SBOM y attestations.

## ÉPICA E11 — Control plane local de telemetría (CP-2026-08)

### SDDK-1101 Cerrar gaps de datos de métricas

**Prioridad:** P0
**Milestone:** CP-2026-08

- Estimar `tokens_used` y `cost_estimate_usd` en la captura automática por modelo (`estimate_cost`).
- Persistir `costs` (L1-L6) cuando el ledger/manifiesto los exponga.
- Poblar `teleological_coherence_pct` desde artifacts del ciclo cuando existan.
- Leer `context_quality` real del `context.json` en lugar del default C2.
- Completar `verify_verdict` con el receipt de verify cuando exista.

**Criterio:** `sddk telemetry status` evidencia >0 ciclos con costos y coherence poblados (o gap documentado).

### SDDK-1102 Store SQLite central del control plane

**Prioridad:** P0
**Milestone:** CP-2026-08

- Schema v1 (`projects`, `cycles`, `aggregates`) en `~/.local/share/sddk/control-plane/control-plane.sqlite`.
- Upsert idempotente por `cycle_id`; proyección reconstruible desde JSONL locales.

**Criterio:** `rm control-plane.sqlite && sddk telemetry ingest` reconstruye el mismo estado; doble ingest no duplica.

### SDDK-1103 Ingest de telemetría cross-proyecto

**Prioridad:** P0
**Milestone:** CP-2026-08

- `sddk telemetry ingest` escanea `projects/*/` (adoption.json + metrics.jsonl + ledger.sqlite).
- Derivación de registros pobres desde eventos del ledger (reuso `derive_from_events`).
- `--dry-run` y `--format json|text`.

**Criterio:** ingest registra todos los proyectos adoptados del host y sus ciclos sin duplicados.

### SDDK-1104 Agregación cross-proyecto

**Prioridad:** P1
**Milestone:** CP-2026-08

- `sddk telemetry aggregate --window 7d|30d` reutilizando `compute_aggregate` sobre el store central.
- Persistencia en `aggregates` + `aggregate.json` + `tuning.md` del control plane.

**Criterio:** aggregate 30d con sample ≥ aggregate 7d cuando existen ciclos de más de 7 días.

### SDDK-1105 Dashboard HTML autocontenido

**Prioridad:** P1
**Milestone:** CP-2026-08

- `sddk telemetry dashboard --output` genera HTML estático sin CDN ni red (patrón `export_html`).
- KPIs, tendencias 7d/30d, distribuciones paths/verdicts, bottleneck por proyecto, señales F3.
- Datasets JSON embebidos; determinista.

**Criterio:** HTML sin URLs externas (grep `https?://` y `src=` externos → vacío), abrible vía `file://`, mismo hash para el mismo store.

### SDDK-1106 Research packet cross-proyecto

**Prioridad:** P1
**Milestone:** CP-2026-08

- `sddk analytics research` alimentado desde el store central cuando exista.
- Research packet con resumen por proyecto (`projects: [...]`).
- Agentes self-research actualizados para consumir el packet cross-proyecto (sin MCP).

**Criterio:** el research packet lista ciclos de todos los proyectos con agregados cross-proyecto.

## ÉPICA E12 — Separación de responsabilidades y cero intrusión (RS-2026-08)

### SDDK-1201 Adopción no intrusiva

**Prioridad:** P0
**Milestone:** RS-2026-08

- Eliminar `plant_workflow_manifest`: `adopt apply` no crea ficheros en el repo.
- El engine resuelve el workflow del manifest embebido o bundle runtime.

**Criterio:** `git status` de un proyecto adoptado queda limpio tras `adopt apply`.

### SDDK-1202 Artefactos de ciclo en XDG

**Prioridad:** P0
**Milestone:** RS-2026-08

- Artefactos de ciclo (proposal, spec, tasks, verify-report, release-report) en `~/.local/share/sddk/projects/<id>/cycle-artifacts/{cycle_id}/`.
- Actualizar `persistence-contract.md`, `openspec-convention.md` y `sdd-kernel-*.md` con los nuevos paths.

**Criterio:** un ciclo completo no deja ficheros bajo el working tree del proyecto.

### SDDK-1203 Generación de docs a XDG

**Prioridad:** P1
**Milestone:** RS-2026-08

- `sddk generate docs|inventory` escribe a `~/.local/share/sddk/projects/<id>/generated/` por defecto.
- Flag `--in-repo` explícito para el dogfooding del repo de desarrollo.

**Criterio:** `sddk generate` en un proyecto no modifica el working tree salvo con `--in-repo`.

### SDDK-1204 Lint sin dependencia del repo

**Prioridad:** P1
**Milestone:** RS-2026-08

- `sddk lint` lee el workflow del manifest embebido/bundle; no exige `workflow/workflow.yaml` en el repo.

**Criterio:** lint pasa en un proyecto sin `workflow/workflow.yaml` en el working tree.

### SDDK-1205 Bundle runtime y dev link

**Prioridad:** P0
**Milestone:** RS-2026-08

- Bundle runtime instalado en `$SDDK_DATA_DIR/framework/<version>/` (modo bundle de `dev update`, modelo asdf `installs/`).
- Múltiples versiones conviviendo; `sddk dev use <version>` actualiza el symlink `current`.
- `dev link`/`dev doctor` operan sobre `current`; los symlinks del editor apuntan ahí, no al repo de desarrollo.

**Criterio:** symlinks de opencode/zcode apuntan bajo `$SDDK_DATA_DIR/framework/current/`; instalar una versión nueva no altera los prompts activos hasta `dev use`.

### SDDK-1206 Migración del estado existente

**Prioridad:** P0
**Milestone:** RS-2026-08

- Eliminar los 2 receipts de adopción duplicados de `.sddk-shared`.
- Mover artefactos de `sddk/` del working tree a XDG.
- Re-linkear opencode/zcode contra el bundle runtime.

**Criterio:** un solo receipt por workspace; `sddk dev doctor` all_present; control plane ingiere identidades únicas.

### SDDK-1207 Resolución de versión por proyecto (modelo asdf)

**Prioridad:** P1
**Milestone:** RS-2026-08

- Resolución de versión: `.sddk-versions` (PWD → padres) → `current` global → `path:<dir>` para dogfooding.
- El framework nunca escribe `.sddk-versions`; lo gestiona el desarrollador (config declarativa, no estado).
- `SDDK_DATA_DIR` env override para todo el árbol de estado.

**Criterio:** un proyecto con `.sddk-versions` usa su versión pin; sin fichero, usa `current`; `path:` apunta al working tree del repo de desarrollo solo cuando se declara explícitamente.

### SDDK-1208 Resolución multiplataforma de paths

**Prioridad:** P1
**Milestone:** RS-2026-08

- Introducir crate `dirs` en `sddk-engine/src/paths.rs`: overrides `XDG_*`/`SDDK_DATA_DIR` primero, fallback `dirs::data_dir()/state_dir()/cache_dir()` por SO.
- macOS → `~/Library/Application Support/sddk`; Windows → `%APPDATA%\sddk`; Linux → XDG (actual).
- Tests de `paths.rs` con caso fallback `dirs`.

**Criterio:** `resolve_xdg_paths` no depende de `HOME` en SO donde no existe (Windows); tests pasan con y sin overrides; `cargo test` verde en linux + darwin.
