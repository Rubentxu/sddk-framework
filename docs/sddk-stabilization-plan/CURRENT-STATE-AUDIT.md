# Auditoría de estado actual — SDDK v3.6

**Fecha:** 2026-08-04
**Baseline Git:** `main` en `0fcb09c`, con cambios modificados y nuevos sin commit
**Veredicto:** implementación fundacional útil, pero v3.6 no está lista para estabilización ni release

## Respuesta ejecutiva

La implementación ha avanzado por delante del roadmap escrito: identidad, adopción, SQLite, ledger, máquina de estados, linter y generación de documentación ya tienen código y pruebas. Sin embargo, esos componentes todavía no forman el runtime autoritativo prometido por el PRD.

El estado aceptado del backlog es:

| Estado | Historias | Porcentaje |
| --- | ---: | ---: |
| Completa | 6 | 19 % |
| Parcial | 10 | 31 % |
| Desviada | 1 | 3 % |
| No iniciada | 15 | 47 % |

Los principales bloqueos no son volumen de código. Son fronteras de autoridad:

1. El trabajo todavía no está versionado ni protegido por CI.
2. El CLI no expone ciclos, fases, ledger, capabilities, reconcile, vault ni release.
3. Los gates son afirmaciones del caller, no evaluaciones autorizadas con receipt.
4. No existe capability gateway; Git, Forge, filesystem y approvals siguen sin frontera runtime.
5. El paquete contiene contratos duplicados que divergen de los contratos raíz.

## Alcance y método

Se contrastaron:

- `PRD.md`, `ROADMAP.md`, `BACKLOG.md`, `MIGRATION.md` y ADR-0001 a ADR-0008.
- `workflow/workflow.yaml` y `schemas/*.json` raíz.
- La copia histórica bajo `docs/sddk-stabilization-plan/`.
- Los cinco crates Rust y sus tests.
- Los contratos legacy de agentes, skills, prompts y shell.
- El estado Git y los gates ejecutables disponibles.

La clasificación exige evidencia en repositorio. Un tipo, tabla o campo aislado no cierra una historia si el criterio requiere integración operativa.

## Hallazgos priorizados

| ID | Severidad | Hallazgo | Acción inmediata |
| --- | --- | --- | --- |
| GAP-001 | P0 | El runtime y los documentos nuevos no están en el historial de `main`; el worktree mezcla varias unidades y contiene `target/` sin trackear. | Preparar una base versionada por work units, excluyendo outputs de build. |
| GAP-002 | P0 | No existe `.github/workflows/`; ningún gate del roadmap se ejecuta automáticamente. | Crear CI mínima antes de declarar PR1-PR4 consolidados. |
| GAP-003 | P0 | El CLI solo expone project/adopt/lint/generate. | Añadir cycle/phase/ledger y conectar Engine + leases + storage. |
| GAP-004 | P0 | `TransitionEvidence.gates` acepta Passed/Failed del caller. | Introducir GateEvaluator, GateReceipt y autorización del emisor. |
| GAP-005 | P0 | Capability gateway, runner y policy engine no existen. | Mantener efectos externos deshabilitados hasta implementar default-deny. |
| GAP-006 | P0 | Dos workflows y dos schemas de agente divergen. | Declarar raíz como única autoridad y archivar o sincronizar snapshots del paquete. |
| GAP-007 | P1 | UUID fallback implementado contradice `fallback: hostname-path` del workflow raíz. | Decidir una semántica y alinear ADR, schema, workflow, código y migración. |
| GAP-008 | P1 | Ledger hash existe, pero no hay CLI verify ni reconstrucción en base vacía. | Cerrar SDDK-402/404 con comandos y fixture de rebuild. |
| GAP-009 | P1 | Frames y leases son primitives sin enforcement extremo a extremo. | Integrarlos en toda mutación y rechazar fencing tokens obsoletos. |
| GAP-010 | P1 | Artifact metadata no es un CAS; SHA-256 es opcional y no se calcula. | Implementar store por contenido y digest obligatorio. |
| GAP-011 | P1 | Receipts permiten insertar directamente estados terminales y JSON sin sanear. | Separar begin/finalize/reconcile y aplicar schemas/redacción. |
| GAP-012 | P1 | Forge/release está corregido solo en prompts y shell. | Implementar puerto Forge, adaptador GitHub y release reconciliable. |
| GAP-013 | P1 | `sddk-testkit` no contiene fixtures ni harness reutilizable. | Implementar fixtures reales antes de cerrar PR2. |
| GAP-014 | P2 | Vault, FTS5, backlinks y petgraph no están iniciados. | Mantener PR8 detrás de PR5-PR7. |
| GAP-015 | P2 | Distribución, receipts, SBOM y attestations no están iniciados. | Mantener PR9 como último corte. |

## Detalle de bloqueos P0

### GAP-001 — No existe una base entregada

El branch actual es `main`, pero `Cargo.toml`, `Cargo.lock`, `crates/`, `schemas/`, `workflow/`, `tests/` y `docs/` aparecen como trabajo nuevo sin commit. Además hay cambios previos en agentes, skills, prompts y bootstrap.

Esto impide afirmar que PR1, PR2, PR3 o PR4 estén entregados. Las pruebas demuestran comportamiento del worktree, no del trunk remoto.

**Criterio de cierre:** commits separados por comportamiento, revisión de archivos incluidos, `target/` excluido, CI verde y trazabilidad entre cada commit y las historias cerradas.

### GAP-002 — Los gates de CI son solo aspiraciones

No existe ningún fichero bajo `.github/workflows/`. Por tanto, el gate de PR2 no está cumplido aunque `sddk lint` y `generate docs --check` funcionen localmente.

La CI mínima debe ejecutar:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run -q -p sddk-cli -- lint --root . --format json
cargo run -q -p sddk-cli -- generate docs --root . --check
bash tests/test_workflow_contract.sh
bash tests/test_adoption_contract.sh
```

### GAP-003 — Rust no es todavía la autoridad operativa

`crates/sddk-cli/src/lib.rs` expone:

- `project resolve`;
- `adopt plan|apply|status|repair`;
- `lint`;
- `generate docs`.

No expone los casos de uso centrales del PRD: `cycle start`, `phase complete`, `ledger verify`, `capability plan/apply`, `reconcile`, vault o release. El engine tiene APIs útiles, pero los prompts todavía poseen la ejecución real.

**Criterio de cierre:** un test end-to-end debe recorrer CLI → Engine → Storage para crear ciclo, completar fase, bloquear/desbloquear, verificar ledger y replay.

### GAP-004 — Los gates no prueban nada por sí mismos

`TransitionEvidence` recibe un mapa de `GateOutcome` suministrado por el caller. El engine comprueba presencia y estado, pero no quién evaluó el gate, qué comando se ejecutó, qué política se usó ni si la evidencia corresponde al plan.

Un gate ejecutable necesita:

- evaluador registrado;
- inputs normalizados;
- actor autorizado;
- hash del plan;
- policy version;
- resultado y evidencia sanitizada;
- receipt vinculado a `command_id` y `frame_id`.

`tests-pass`, `policy-compliant`, `review-approved`, `no-pending-effects` y `ledger-valid` no deben aceptar autoafirmación del mismo caller que solicita la transición.

### GAP-005 — Falta la frontera de capacidades

ADR-0005 exige `validate → plan → authorize → apply → verify → receipt`. El workspace solo contiene tipos, metadata y almacenamiento idempotente de receipts.

No existen:

- runner con argv tipado y allowlist de entorno;
- filesystem restringido y seguro frente a symlinks/escapes;
- Git local;
- policy decision point;
- approvals R3/R4;
- lifecycle `started → succeeded|failed|unknown`;
- reconciliación.

Esta ausencia es un bloqueador de diseño antes de habilitar operaciones mutantes o externas. No se clasifica como vulnerabilidad remota activa porque el gateway todavía no está expuesto.

### GAP-006 — Contratos duplicados y divergentes

| Contrato | Raíz actual | Copia del paquete | Drift |
| --- | --- | --- | --- |
| Workflow | Paths A-min/A-lite/A-full/B-direct, transiciones path-scoped, artifacts y gates | Workflow lineal sin paths y `debt_verification: optional` | La copia no representa el runtime actual. |
| Requirement | String o `{kind, name}` | Usa `{artifact: value}` / `{gate: value}` | Shape incompatible con el dominio actual. |
| Agent result | Artifacts/capabilities opcionales; `relation_type` | Ambos obligatorios; usa `type` | Resultados válidos en raíz pueden fallar en la copia. |
| Project fallback | `hostname-path` en workflow | UUID en backlog/ADR/código | Autoridad sin resolver. |

**Recomendación:** mantener `workflow/workflow.yaml` y `schemas/*.json` como únicos contratos ejecutables. Las copias del paquete deben convertirse en snapshots históricos claramente marcados o eliminarse tras conservar su procedencia.

## Gaps P1 de implementación

### Ledger, replay y concurrencia

- `verify_ledger` detecta huecos y manipulación, pero no tiene comando CLI.
- `replay_cycle` reconstruye el último snapshot desde eventos existentes; no repuebla una base vacía.
- `frame_id` se persiste, pero no se impone la relación command → frame.
- Los leases tienen fencing token, pero ninguna operación del engine exige el token.
- No existe renovación, inspección o recuperación de lease desde CLI.
- Solo existe schema SQLite v1; no hay estrategia probada de backup/migración v2+.

### Artefactos y receipts

- `insert_artifact` almacena metadata; no recibe bytes ni calcula el digest.
- `sha256` es opcional y no existe unicidad/deduplicación por contenido.
- `record_capability_receipt` permite insertar `succeeded` directamente.
- No existe API para finalizar o reconciliar un receipt iniciado.
- Requests, results, payloads y snapshots aceptan JSON arbitrario sin redacción; podrían persistir secretos o PII.
- `actor` y timestamps son caller-supplied, por lo que la atribución es declarativa.

### Forge y release

- `ForgeDef` es configuración, no un trait.
- No existe adaptador GitHub runtime.
- `CapabilityStatus::Unknown` no tiene reconciliador.
- Las 117 pruebas shell protegen el contrato textual de release, no ejecutan una release Rust.
- No hay rollback side-by-side ni receipt de promoción.

### Calidad de la base

- `sddk-testkit` no ofrece fixtures, mocks ni harness reutilizable.
- README mantiene inventario manual: 63/89 declarados frente a 64 agentes y 90 `SKILL.md` actuales.
- Los schemas existen, pero ningún JSON Schema validator se ejecuta en runtime.

## Estado por PR

| PR | Implementación observada | Estado de aceptación |
| --- | --- | --- |
| PR 1 | Hotfix semántico, adopción legacy y release textual corregidos | Parcial; sin inventario generado, contrato único ni entrega versionada. |
| PR 2 | Cinco crates, linter y generador funcionales | Parcial; sin CI, fixtures de testkit ni validación completa de schemas. |
| PR 3 | Identidad, XDG y adopción reparable | Funcional, pero bloqueado por drift UUID/hostname-path. |
| PR 4 | SQLite, hash chain, engine, replay y leases | Parcial; APIs internas sin superficie CLI ni integración de concurrencia. |
| PR 5 | Receipts y artifact metadata como foundations | No iniciado como gateway. |
| PR 6 | AgentResult y schema | Parcial; adapter y permisos no iniciados. |
| PR 7 | Contrato legacy de release con tests | Parcial; Forge/reconcile runtime no iniciados. |
| PR 8 | ADRs y templates | No iniciado. |
| PR 9 | ADR de distribución | No iniciado. |

## Verificación ejecutada

| Gate | Resultado |
| --- | --- |
| `cargo test --workspace` | PASS, 85 tests. |
| `sddk lint --format json` | PASS, 0 errores y 0 warnings. |
| `sddk generate docs --check` | PASS, documentación actual. |
| `tests/test_workflow_contract.sh` | PASS, 117 checks. |
| `tests/test_adoption_contract.sh` | PASS, 22 checks. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS tras retirar el placeholder inválido. |
| CI remota | AUSENTE. |

## Plan de acción recomendado

### Work unit A — Canon y baseline

**Objetivo:** convertir el worktree actual en una base revisable sin mezclar outputs ni contratos contradictorios.

**Acciones:**

1. Resolver UUID vs `hostname-path`.
2. Declarar root workflow/schemas como autoridad única.
3. Marcar o retirar snapshots incompatibles del paquete.
4. Corregir `.gitignore` para excluir `target/`.
5. Separar commits PR1, PR2, PR3 y PR4 por comportamiento.

**Gate:** checkout limpio reproduce todos los gates locales.

### Work unit B — CI y testkit

**Objetivo:** hacer reales los gates ya implementados.

**Acciones:** CI mínima, testkit con fixture repository/workflow/storage, Clippy estricto y checks locked.

**Gate:** required check único o conjunto documentado, obligatorio antes de merge.

### Work unit C — Cierre PR4

**Objetivo:** exponer la autoridad local ya construida.

**Acciones:** CLI cycle/phase/ledger, frame invariant, leases/fencing, replay rebuild y errores estables.

**Gate:** test end-to-end sin red ni reloj real que recorra un ciclo y reconstruya estado.

### Work unit D — Capability gateway

**Objetivo:** crear una frontera no eludible antes de Git/Forge.

**Acciones:** policy default-deny, approvals vinculadas al plan, begin/finalize/reconcile receipt, runner tipado, filesystem seguro y redacción.

**Gate:** pruebas negativas demuestran que R3/R4, shell arbitrario, path escape y gate autoafirmado son rechazados.

### Work unit E — Git, CAS y agentes

**Objetivo:** cerrar PR5 y PR6 sobre el gateway.

**Acciones:** Git local, CAS SHA-256, adapter legacy, schema validation y permisos agent/phase/capability.

**Gate:** toda acción produce receipt, postcondición verificada y evento causal.

### Work unit F — Forge/release

**Objetivo:** mover la secuencia ya estabilizada desde prompts a runtime reconciliable.

**Acciones:** trait Forge, GitHub adapter, unknown reconciliation, release plan/apply/reconcile y rollback.

**Gate:** interrupciones simuladas entre merge/tag/publish no duplican efectos y convergen tras reconcile.

### Work units G-H — Vault y distribución

Ejecutar PR8 y PR9 solo después de cerrar los work units anteriores. LadybugDB permanece fuera de v3.6.

## Decisiones que requieren cierre explícito

| Decisión | Opciones | Recomendación |
| --- | --- | --- |
| Fallback sin remote | UUID persistido vs `hostname-path` | UUID persistido: evita que mover el checkout cambie la identidad. Actualizar workflow/ADR. |
| Workflow del paquete | Copia ejecutable vs snapshot | Snapshot histórico o referencia; no mantener dos contratos ejecutables. |
| Validación de gates | Caller assertion vs receipt autorizado | Receipt autorizado y vinculado al plan. |
| Vault canónico | Paths XDG del runtime vs vault de conocimiento existente | Separar explícitamente estado operativo XDG de conocimiento canónico; documentar ownership y migración. |
| Migración SQLite | Auto-migrate al abrir vs comando explícito | Backup + lock exclusivo + migración explícita para cambios destructivos. |

## Criterio actualizado de salida v3.6

v3.6 no debe declararse estable hasta que:

- el trabajo esté versionado y CI sea obligatoria;
- Rust controle adopción, ciclo, fase, gates, Git local, ledger y recuperación;
- ningún gate crítico acepte una afirmación no autorizada;
- toda capacidad mutante pase por gateway, policy y receipt;
- replay/reconcile recuperen interrupciones sin editar SQLite manualmente;
- Forge/release converja sin duplicar efectos;
- no existan contratos ejecutables duplicados o contradictorios.
