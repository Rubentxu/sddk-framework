# Backlog técnico — SDDK v3.6

**Estado auditado:** 2026-08-04
**Baseline:** `v0.2.0`
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> El estado mide criterios de aceptación demostrados en el repositorio actual. Código presente sin integración, gate automático o evidencia suficiente se marca como parcial. Nada de este paquete se considera entregado hasta quedar versionado y protegido por CI.

## Resumen de estado

| Estado | Historias | Significado |
| --- | ---: | --- |
| Completa | 28 | Todos los criterios de la historia tienen implementación y prueba directa. |
| Parcial | 0 | No queda ninguna historia parcial. |
| Desviada | 0 | No queda ninguna desviación contractual conocida. |
| No iniciada | 4 | No existe implementación runtime suficiente. |
| **Total** | **32** | La base Rust es funcional, pero v3.6 todavía no cumple su criterio de salida. |

## Matriz de aceptación

| Historia | Estado | Evidencia actual | Gap que impide cerrarla |
| --- | --- | --- | --- |
| SDDK-101 | Parcial | `workflow/workflow.yaml`; `validate_workflow`; tests de dominio; snapshots duplicados retirados | No se ejecuta validación JSON Schema completa. |
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
| SDDK-902 | Completa | Índice FTS5 reconstruible (drop → reindex) y backlinks derivados | Sin gap funcional demostrado. |
| SDDK-903 | Completa | Validación VAULT001-VAULT004: ids, títulos y wikilinks rotos | Sin gap funcional demostrado. |
| SDDK-904 | Completa | Grafo `petgraph`: ciclos, camino de muestra y orden topológico | Sin gap funcional demostrado. |
| SDDK-1001 | No iniciada | `bootstrap.sh` solo instala enlaces legacy | Falta `xtask` o equivalente con fmt/clippy/test/install/doctor. |
| SDDK-1002 | No iniciada | Sin receipt de instalación | Falta versión, commit, hash, canal y promoción verificable. |
| SDDK-1003 | No iniciada | Sin pipeline de release | Faltan binarios, checksums, SBOM y attestations. |

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
