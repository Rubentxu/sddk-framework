# Backlog técnico — SDDK v3.6

**Estado auditado:** 2026-08-04
**Baseline:** `main` en `0fcb09c` más cambios sin commit en el worktree
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> El estado mide criterios de aceptación demostrados en el repositorio actual. Código presente sin integración, gate automático o evidencia suficiente se marca como parcial. Nada de este paquete se considera entregado hasta quedar versionado y protegido por CI.

## Resumen de estado

| Estado | Historias | Significado |
| --- | ---: | --- |
| Completa | 6 | Todos los criterios de la historia tienen implementación y prueba directa. |
| Parcial | 10 | Existe una base útil, pero falta al menos un criterio o integración obligatoria. |
| Desviada | 1 | La implementación existe, pero contradice otro contrato canónico. |
| No iniciada | 15 | No existe implementación runtime suficiente. |
| **Total** | **32** | La base Rust es funcional, pero v3.6 todavía no cumple su criterio de salida. |

## Matriz de aceptación

| Historia | Estado | Evidencia actual | Gap que impide cerrarla |
| --- | --- | --- | --- |
| SDDK-101 | Parcial | `workflow/workflow.yaml`; `validate_workflow`; tests de dominio | No se ejecuta validación JSON Schema completa y la copia del paquete está obsoleta. |
| SDDK-102 | Parcial | `sddk generate docs`; SDDK009; tests deterministas | No existe CI que ejecute `generate docs --check`. |
| SDDK-201 | Completa | SDDK001 y tests de referencias tipadas | Sin gap funcional de historia; falta automatizarla en CI a nivel roadmap. |
| SDDK-202 | Completa | SDDK002-SDDK004 y fixtures de shell ejecutable | Sin gap funcional de historia; el escaneo de fences es opt-in por diseño. |
| SDDK-203 | No iniciada | README mantiene conteos manuales | Falta inventario generado y comprobación de drift. |
| SDDK-301 | Desviada | Identidad remote/scope/UUID y tests | El código y este backlog usan UUID; `workflow.project_identity.fallback` declara `hostname-path`. |
| SDDK-302 | Completa | Receipt v2, hash de configuración y rename atómico | Sin gap funcional demostrado. |
| SDDK-303 | Completa | `adopt repair`; tests ReceiptOnly/LedgerOnly/conflicto/corrupción | Sin gap funcional demostrado. |
| SDDK-401 | Completa | SQLite v1, WAL, foreign keys y migración transaccional | La evolución v2+ queda como riesgo del roadmap, no de este criterio inicial. |
| SDDK-402 | Parcial | Cadena hash, triggers append-only y `verify_ledger` | Falta el comando `sddk ledger verify` y una prueba CLI de corrupción. |
| SDDK-403 | Parcial | `frame_id` y `command_id` se persisten | Falta imponer que todos los eventos de un comando compartan frame y consultar por frame. |
| SDDK-404 | Parcial | Replay y comparación contra snapshot materializado | No reconstruye una base vacía ni expone replay/rebuild por CLI. |
| SDDK-501 | Completa | Rechazo de transición, source, artifacts, gates y paths | Sin gap funcional de la API de engine; todavía no está expuesta por CLI. |
| SDDK-502 | Parcial | Leases con owner, expiry y fencing token | El engine/CLI no adquiere, renueva ni aplica el fence en cada mutación. |
| SDDK-601 | No iniciada | Solo existe `CapabilityRequest` como contrato | Falta runner tipado, allowlist de entorno, límites y captura sanitizada. |
| SDDK-602 | Parcial | Escrituras atómicas de adopción/docs y paths XDG | Falta gateway filesystem reutilizable, canonicalización y defensa frente a escapes/symlinks. |
| SDDK-603 | No iniciada | Solo lectura fija de `git config --get remote.origin.url` | Faltan inspect/branch/commit/tag y verificación de postcondiciones. |
| SDDK-604 | Parcial | Metadata de artefactos en SQLite | Falta CAS real, SHA-256 obligatorio, deduplicación y verificación de bytes. |
| SDDK-701 | Parcial | Schema y `AgentResult` tipado | El runtime no valida JSON Schema y la copia del paquete diverge del schema raíz. |
| SDDK-702 | No iniciada | Sin adaptador runtime | Falta convertir resultados legacy y emitir campos no verificables. |
| SDDK-703 | No iniciada | Riesgos declarativos en workflow | Falta política agent + phase + capability y enforcement default-deny. |
| SDDK-801 | No iniciada | `ForgeDef` es solo configuración | Falta trait neutral, mock y pruebas de contrato. |
| SDDK-802 | No iniciada | Release legacy vive en prompts/scripts | Falta adaptador GitHub runtime para PR/checks/merge/release. |
| SDDK-803 | No iniciada | Existe `CapabilityStatus::Unknown` | Falta lifecycle de receipts y reconciliación contra el proveedor. |
| SDDK-804 | Parcial | Orden de release corregido y 117 checks contractuales | Sigue siendo lógica de prompts/shell; no existe enforcement Rust. |
| SDDK-901 | No iniciada | Solo templates y ADR | Falta parser de frontmatter, IDs, tipos, relaciones y procedencia. |
| SDDK-902 | No iniciada | Sin migraciones FTS5/backlinks | Falta índice reconstruible e incremental. |
| SDDK-903 | No iniciada | Sin validador de relaciones | Faltan errores estables y tests de referencias/tipos inválidos. |
| SDDK-904 | No iniciada | `petgraph` no es dependencia | Faltan proyección, ciclos, caminos y orden topológico. |
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
