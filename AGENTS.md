# AGENTS.md — sddk-framework

> Convenciones, layout de directorios y reglas que todo agente (humano o
> IA) debe respetar al trabajar en este repo. Léelo antes de hacer
> cambios — la separación de directorios es **estructural**, no
> cosmética, y romperla contamina el bundle runtime.

---

## 1. Contexto del proyecto

`sddk-framework` es el **repo de desarrollo** (NO adoptado) del framework
SDDK. Contiene crates, docs, CI, releases, agents/skills/prompts **fuente**.
Aquí se hace el commit; aquí se revisa el PR; aquí se etiqueta el release.

El proyecto **nunca escribe dentro de otros repos de proyectos** (regla
"cero intrusión", ver `docs/responsibility-separation/SPEC.md`).
El bundle runtime se publica en `$SDDK_DATA_DIR/framework/<version>/`
(generalmente `~/.local/share/sddk/framework/<v>/`) vía `sddk dev install`.

El **proyecto vive en el directorio actual** (`$(pwd)` =
`~/Proyectos/agentesIA/sddk-framework`). Todo cambio, commit, push y
release se hace desde aquí. Ver §3 para el layout completo.

---

## 2. Convenciones duras (no negociables)

### 2.0. Frontera de namespace

- Gentle AI SDD y SDDK son sistemas distintos. Sus agentes, skills, prompts y
  contratos de persistencia no se mezclan.
- El nombre historico "SDD-kernel" designaba este mismo flujo y queda
  normalizado a **SDDK**.
- La unica superficie activa de este framework es `orchestrator`, `sddk-*` y
  `prompts/sddk/`. No se crean aliases `sdd-*`, `sdd-kernel-*` ni
  `gentle-orchestrator`.

### 2.1. Commits

- **Conventional Commits** en español: `feat(uat): …`, `fix(uat): …`,
  `chore(release): …`. Sin `Co-Authored-By` ni atribución a IA.
- Una concernencia por commit. Si un cambio toca docs + código, un solo
  commit con la concernencia explicada en el body.
- Commits pusheados con `git push origin main` (no PRs — es un proyecto
  lineal en `main` con tags `vX.Y.Z`).

### 2.2. Branch model

- `main` es la rama única de desarrollo + releases. No hay `develop`,
  `release/*`, ni hotfix branches. Tags `vX.Y.Z` marcan releases.
- Cualquier feature se commitea directo a `main` (o se squash-margea en
  PRs externos, no es el flujo habitual).

### 2.3. Workspace

- `Cargo.toml` `[workspace.package] version` = versión de desarrollo
  actual (puede ir ahead del último tag hasta que se haga `chore(release)`).
- `cargo test --workspace` debe pasar verde antes de commitear.
- `cargo clippy --workspace` debe pasar sin errores (warnings de
  unused son aceptables, errores no).

### 2.4. Memory + Engram

- Sesiones largas (varias horas, contexto pesado) DEBEN cerrar con
  `engram_mem_session_summary` al final, describiendo:
  - Goal (qué se estaba haciendo)
  - Discoveries (gotchas, decisiones, edge cases)
  - Accomplished (qué se entregó, archivos tocados)
  - Next Steps (qué queda para futuras sesiones)
- Esto sobrevive compactaciones de contexto. Sin esto, la siguiente
  sesión arranca en frío.
- Reglas completas en `.opencode` / `~/.config/opencode/skills/...`.

---

## 3. Layout de directorios (modelo asdf-vm)

Inspirado en `asdf-vm` (tool versions, shims por versión, `path:` override).
El spec canónico está en `docs/responsibility-separation/SPEC.md`.

### 3.1. Tres roles separados

| Rol | Ubicación | Contenido | Adoptado | Linkado |
|-----|-----------|-----------|----------|---------|
| **Repo de desarrollo** | `~/Proyectos/agentesIA/sddk-framework/` (CWD) | `crates/`, `docs/`, `agents/`, `skills/`, `prompts/`, CI, releases | NO | NO |
| **Bundle runtime** | `$SDDK_DATA_DIR/framework/<version>/` (típicamente `~/.local/share/sddk/framework/v1.5.3/`) | Snapshot del release: `agents/`, `skills/`, `prompts/`, `workflows/`, `assets/` | — | SÍ (a `$HOME/.config/{opencode,claude,kilo,codex}/`) |
| **Workspace de uso** | Repos reales de proyectos del usuario | Código del proyecto + opcional `.sddk-versions` | SÍ | NO |

### 3.2. Resolución de versión (modelo asdf, lookup en orden)

1. `$PWD/.sddk-versions`
2. `.sddk-versions` en directorios padre hasta la raíz
3. `$SDDK_DATA_DIR/framework/current` (global, symlink a la versión activa)

Formato de `.sddk-versions` (gestionado por el **desarrollador**, NUNCA por el framework):

```text
sddk 1.5.3
sddk current         # sigue el symlink global
sddk path:../..      # dogfooding del repo de desarrollo (CWD = sddk-framework)
sddk system          # instalación del sistema (brew, etc.) — opcional
```

### 3.3. Cero intrusión

| Operación | Antes (mal) | Ahora (bien) |
|-----------|------------|--------------|
| Adopción | `workflow/workflow.yaml` plantado en repo | nada; receipt en `~/.local/share/sddk/projects/<id>/` |
| Artefactos de ciclo | `sddk/{change}/...` en repo | `~/.local/share/sddk/projects/<id>/cycle-artifacts/{cycle_id}/` |
| Docs generados | `docs/generated/` en repo | `~/.local/share/sddk/projects/<id>/generated/` (o `--in-repo` explícito solo para dogfooding) |
| Telemetry / control plane | `~/.local/share/sddk/uat-results.sqlite` | siempre XDG, nunca en repo |

---

## 4. Reglas de oro

### 4.1. Trabajar SIEMPRE desde el CWD (`sddk-framework/`)

- ✅ `cd ~/Proyectos/agentesIA/sddk-framework && git … && cargo …`
- ❌ `cd ~/.sddk-shared/ && …` — esto era un **segundo checkout** que existía
  por hábito pero que viola la regla "single source of truth en el CWD".
  Está marcado para eliminación. **No crear nuevos checkouts en
  `~/.sddk-shared/`.** Si te encuentras trabajando ahí, muévete al CWD.

### 4.2. El bundle runtime vive en `~/.local/share/sddk/framework/<v>/`

- Se actualiza con `sddk dev install` (o `sddk dev update`).
- **No es un checkout de git.** Es un snapshot publicado.
- Si modificas un asset en `assets/uat-dashboard/...` (CWD), debes correr
  `sddk dev install` o `sddk dev update` para que el bundle runtime
  lo recoja. **No edites directamente `~/.local/share/sddk/...`** — los
  cambios se sobrescriben en el próximo install.

### 4.3. El bundle runtime NO es un checkout del repo

- `~/.local/share/sddk/framework/<v>/agents/`, `skills/`, `prompts/` son
  **copias** del release, no symlinks. `bootstrap.sh` los symlinkea a
  los directorios de cada editor (`~/.config/opencode/`, `~/.claude/`...).

### 4.4. Las decisiones de diseño viven en `docs/adr/` o `~/.sddk-knowledge/`

- `docs/adr/` (este repo) — ADRs que afectan al proyecto público.
- `~/.sddk-knowledge/<project>/adrs/` — ADRs que afectan a un proyecto
  adoptado específico. Ejemplo: `~/.sddk-knowledge/sddk-framework/adrs/`.
- Specs del plan viven en `~/.sddk-knowledge/<project>/specs/`.

---

## 5. Regresiones detectadas (a resolver en futuras sesiones)

Estas cosas **no se arreglaron todavía** pero violan el spec. Documentadas
para no perderlas de vista.

### 5.1. `~/.sddk-shared/` (REGRESIÓN RESUELTA — 2026-08-08)

`~/.sddk-shared/` era un **segundo checkout** del mismo repo. Todo el
trabajo de desarrollo debe ocurrir en el CWD (`sddk-framework/`).

**Resuelto**: eliminado con `rm -rf /var/home/rubentxu/.sddk-shared/`
previa verificación de que los 3 commits y los 4 cambios uncommitted
estaban en el CWD / `origin/main` (ver commit `98b20d7` que documenta
esta regresión retrospectivamente).

**Prevención**: no vuelvas a crear un segundo checkout. Si necesitas
iterar, usa el CWD. El bundle runtime (`~/.local/share/sddk/framework/v1.5.3/`)
se actualiza con `sddk dev install` (ver §4.2).

### 5.2. `bootstrap.sh` referencia `~/.sddk-shared/` (DRIFT)

```
$ grep -c sddk-shared bootstrap.sh
1
```

**Problema**: el bootstrap dice *"make `~/.sddk-shared/` the single source
of truth"*, pero el spec RS-2026-08 cambió la fuente de verdad al CWD
(`sddk-framework/`) + bundle runtime en `~/.local/share/sddk/framework/`.

**Acción**: actualizar `bootstrap.sh` para:
- Usar `$(cd "$(dirname "$0")" && pwd)` (que YA es lo que hace para
  `SHARED_DIR`) como source de agents/skills/prompts.
- Apuntar los symlinks a `~/.local/share/sddk/framework/current/`
  (no a `~/.sddk-shared/`).
- Renombrar/desinstalar la variable `SDDK_SHARED_DIR`.

### 5.3. `Cargo.toml` v1.5.3 sin tag público (DRIFT menor)

```
[workspace.package]
version = "1.5.3"
```

Los últimos tags en `origin` son `v1.5.0`, `v1.5.2`, `v1.5.3`. El HEAD
actual está ahead del último tag público porque incluye commits de
desarrollo sin taggear. **No es un bug** — es el flujo normal —
pero requiere `chore(release): bump to vX.Y.Z` cuando se quiera
publicar.

---

## 6. Checklist antes de commitear

```text
[ ] cargo build --release -p sddk-cli            # compila
[ ] cargo test --workspace                       # 215 verde
[ ] cargo clippy --workspace                    # 0 errores
[ ] Si tocaste assets/: sddk dev install        # bundle runtime actualizado
[ ] git status                                  # clean
[ ] git diff                                    # revisas lo que vas a commitear
[ ] commit mensaje: feat(uat): … o fix(uat): …
[ ] git push origin main                        # pusheas
```

---

## 7. Recovery — si algo se rompe

### "Compilé cambios en `~/.sddk-shared/` y los perdí"

```bash
# 1. Verificar que los cambios están en origin
cd ~/.sddk-shared && git log --oneline -5
# 2. Traerlos al CWD
cd ~/Proyectos/agentesIA/sddk-framework
git fetch origin
git log --oneline HEAD..origin/main
git merge --ff-only origin/main
```

### "El bundle runtime está desactualizado"

```bash
sddk dev install   # copia los assets/ actuales al runtime
```

### "Los tests pasan en CWD pero fallan en runtime"

Probablemente editaste `~/.local/share/sddk/framework/<v>/…` directamente.
Re-corre `sddk dev install` para sobrescribir con la versión de CWD.

### "Olvidé en qué directorio estoy"

```bash
pwd
# siempre debe ser /var/home/rubentxu/Proyectos/agentesIA/sddk-framework
# Si no lo es, muévete: cd /var/home/rubentxu/Proyectos/agentesIA/sddk-framework
```

---

## 8. Resumen en una línea

> **El proyecto es el CWD** (`sddk-framework/`). El bundle runtime vive
> en `~/.local/share/sddk/framework/<v>/` (instalado por `sddk dev install`).
> `~/.sddk-shared/` era un segundo checkout **eliminado el 2026-08-08**
> (drift del modelo asdf-vm). Todo cambio de código va al CWD; todo cambio de
> contenido publicable se copia al bundle con `sddk dev install`.

---

## 9. Session handoff (2026-08-08) — qué pasó + dónde seguir

Para que la próxima sesión sepa exactamente dónde está el proyecto sin
re-descubrirlo. Lee esto ANTES de tocar nada.

### Estado al cierre (HEAD = `f3fb9c9`)

- **215 tests verde**, 0 clippy errors (`cargo test --workspace && cargo clippy --workspace`).
- **6 commits ahead** del release público v1.5.3 (tag en `origin`); todos
  en `main` y pusheados a `origin/main`:
  - `d33d102` fix(uat): uat history accepts --sessions X Y (positional)
  - `1cce878` fix(uat): collapse nested if-let (clippy)
  - `ea66d58` feat(uat): v2 schema — plan+session, XDG-resident manifest, typed evidence, history aggregator, wizard v2
  - `4c174df` feat(uat): wire wizard to in-process ingest server (closes dashboard → control plane loop)
  - `98b20d7` docs(agents): AGENTS.md — directory layout (asdf-vm inspired) + detected regressions
  - `f3fb9c9` fix(docs): replace all .sddk-shared/ paths with CWD + XDG bundle runtime
- **`~/.sddk-shared/` ELIMINADO** (33G, segundo checkout del mismo repo).
- **`Cargo.toml` version = "1.5.3"** (sin tag público para los 6 commits; tag
  `v1.5.4` se puede crear con `chore(release): bump to v1.5.4`).
- **Bundle runtime** ya sincronizado: `~/.local/share/sddk/framework/v1.5.3/`
  tiene los assets con md5 idéntico al CWD.

### Lo que está implementado (no rehacer)

✅ **Plan v2**: `context.{user_story, preconditions, workspace, timing, help, failure_protocol, postconditions, test_data}`, `evidence.kinds` tipados, `risk.{classification, blast_radius, mitigation}`, `automation`, `provenance`.
✅ **Session v2**: `metadata.{tester, env_fingerprint, build, duration_ms}`, per-result `verdict_at / verdict_duration_ms / tester_notes / observed / failure_reason / linked_defect / repro_command`.
✅ **XDG manifest + verify-integrity**: SHA-256 self-contained (NIST-verified), exit 0/0/1 para ok/partial/fail.
✅ **History aggregator**: per-scenario runs/passing/failing/blocked + success_rate + flakiness_score + first/last_run + defect_ids + avg/p95_duration + trend (improving/degrading/stable).
✅ **Wizard v2** (browser): pre-flight checklist, sticky context bar (window/est-ceiling/risk/help), typed steps, typed evidence capture, failure protocol flow, teardown checklist, persistent tester id `T-XXXX`.
✅ **Wired dashboard → control plane**: `sddk uat open` arranca HTTP server en `127.0.0.1:0`, wizard POSTea `/ingest`, server cierra con Ctrl+C via `AtomicBool` shutdown flag. Mismo origen (GET / sirve el wizard HTML) → sin CORS.
✅ **Suggester + apply**: `sddk uat scenario-context --plan FILE [--apply]` — reglas deterministas (timing desde est_minutes, preconditions desde step kind, risk desde priority, evidence default Note, automation Manual, provenance desde plan metadata). Subjectivos (`user_story`) se quedan como placeholder.
✅ **Documentation**: `docs/uat/EXAMPLE-uat-plan-v2.md` (plan completo de ejemplo), `~/.sddk-knowledge/sddk-framework/adrs/ADR-012-uat-human-in-the-loop.md` (§4+§7 actualizados), `AGENTS.md` (este fichero), `~/.sddk-knowledge/sddk-framework/cycles/CYC-UAT-V2-uat-schema-history-wiring.md` (cycle manifest completo con handoff).

### Lo que queda pendiente (opcional, en orden de prioridad)

1. **`uat history --view timeline` HTML view** — diferido de P5. Renderizaría timeline por escenario + heatmap de flakiness + linkage de defects. Si no, el CLI ya da la respuesta "6 months later" en YAML.
2. **`chore(release): bump to v1.5.4` + tag** — para marcar el estado actual del CWD. NO requiere código nuevo, solo un commit + `git tag v1.5.4 && git push --tags`.
3. **`bootstrap.sh` cosmético** — la variable `SHARED_DIR` (línea 16) tiene nombre misleading; apunta al CWD pero el nombre sugiere "shared dir" antiguo. Renombrar a `SDDK_FRAMEWORK_ROOT` + actualizar comentario. Funcionalmente correcto hoy, solo confuso para quien lo lea.
4. **`docs/responsibility-separation/SPEC.md`** — añadir sección "current state" mostrando el antes/después de la eliminación de `~/.sddk-shared/`. Hoy el spec describe el "antes" como referencia; útil documentar el "después" para audit.
5. **Auto-runner (`sddk uat run --scenario X`)** — diferido en P3 como v3. El hook `automation.{status, ref, ci_job, when}` ya está en el schema. Runner ejecutaría scripts referenciados por `automation.ref`. Próximo ciclo potencial.
6. **UPDATE `~/.sddk-knowledge/sddk-framework/adrs/ADR-0011-...`** — actualmente describe `~/.sddk-shared/` como "estado actual pre-fix" (correcto históricamente). Podría añadirse una sección "estado resuelto 2026-08-08" para audit trail.

### Memoria de la sesión (para grep rápido)

Toda la memoria detallada de los P0-P5 + wiring + directory-confusion fix está en Engram bajo la sesión `sddk-framework`. Próxima sesión: `mem_context` con project="sddk-framework" + `mem_search` con keywords (uat, v2, schema, wizard, history, .sddk-shared, asdf-vm) recupera los resúmenes de esta sesión.

### Cómo reabrir esta sesión

```bash
cd ~/Proyectos/agentesIA/sddk-framework
cargo test --workspace        # debe dar 215 verde
cargo clippy --workspace      # 0 errors
git log --oneline -8          # los 6 commits del CYC
ls ~/.local/share/sddk/framework/v1.5.3/   # bundle runtime
cat ~/.sddk-knowledge/sddk-framework/cycles/CYC-UAT-V2-uat-schema-history-wiring.md   # handoff completo
```

Si todo coincide, el proyecto está sano. Si algún test falla, comparar con el
log de la sesión en Engram antes de tocar nada.
