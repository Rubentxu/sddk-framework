# AGENTS.md — sddk-framework

> Convenciones, layout y reglas que todo agente (humano o IA) debe respetar.
> Léelo antes de hacer cambios — la separación de directorios es **estructural**,
> no cosmética, y romperla contamina el bundle runtime.

---

## 1. Contexto del proyecto

`sddk-framework` es el **repo de desarrollo** (NO adoptado) del framework SDDK.
Contiene crates, docs, CI, releases, agents/skills/prompts **fuente**. Todo cambio,
commit, push y release se hace desde `~/Proyectos/agentesIA/sddk-framework/` (CWD).

El proyecto **nunca escribe dentro de otros repos de proyectos** (regla "cero
intrusión", ver `docs/responsibility-separation/SPEC.md`). El bundle runtime
vive en `$SDDK_DATA_DIR/framework/<version>/` (`~/.local/share/sddk/framework/<v>/`)
y se actualiza con `sddk dev install`.

---

## 2. Convenciones duras (no negociables)

### 2.0. Frontera de namespace

- Gentle AI SDD y SDDK son sistemas distintos. Sus agentes, skills, prompts y
  contratos de persistencia no se mezclan.
- El nombre historico "SDD-kernel" queda normalizado a **SDDK**.
- La superficie activa es `orchestrator`, `sddk-*` y `prompts/sddk/`. Sin aliases.

### 2.1. Commits

- **Conventional Commits** en español: `feat(uat): …`, `fix(uat): …`, `chore(release): …`.
  Sin `Co-Authored-By` ni atribución a IA.
- Una concernencia por commit. Si un cambio toca docs + código, un solo commit con la
  concernencia explicada en el body.
- Commits a `main` via `git push origin main` (no PRs — proyecto lineal con tags `vX.Y.Z`).

### 2.2. Branch model

- `main` es la rama única de desarrollo + releases. No hay `develop`, `release/*` ni hotfix.
- Cualquier feature se commitea directo a `main` (o se squash-margea en PRs externos).

### 2.3. Workspace

- `Cargo.toml` `[workspace.package] version` = versión de desarrollo actual (puede ir
  ahead del último tag hasta `chore(release)`).
- `cargo test --workspace` verde + `cargo clippy --workspace` sin errores antes de commitear.

### 2.4. Memory + Engram

- Sesiones largas DEBEN cerrar con `engram_mem_session_summary` (goal, discoveries,
  accomplished, next steps, relevant files). Sobrevive compactaciones. Reglas en
  `~/.config/opencode/skills/...`.

### 2.5. CI local-first, cloud async

- **El gate de verificación es LOCAL**: `cargo test --workspace` + `cargo clippy
  --workspace` + fmt antes de commitear (ver checklist §5). GitHub Actions cloud
  **NO bloquea**: sin required status checks, runs = evidencia asíncrona.
- **Prohibido** esperar runs de la nube (`gh pr checks --watch`, retrasar
  push/merge por CI) o "arreglar CI" sin reproducir en local primero.
- **Workflows en local**: `act` v0.2.89 (`/usr/local/bin/act`) + podman;
  `ubuntu-latest` mapeado a `catthehacker/ubuntu:rust-latest` vía
  `~/.config/act/actrc`. Ejemplo: `act pull_request -W .github/workflows/<wf>.yml`.
- Los minutos del plan free de GitHub están agotados — el cloud puede ni
  ejecutar; confía en el gate local.

---

## 3. Layout de directorios (modelo asdf-vm)

Inspirado en `asdf-vm` (tool versions, shims por versión, `path:` override).
El spec canónico: `docs/responsibility-separation/SPEC.md`.

### 3.1. Tres roles separados

| Rol | Ubicación | Contenido | Adoptado | Linkado |
|-----|-----------|-----------|----------|---------|
| **Repo de desarrollo** | `~/Proyectos/agentesIA/sddk-framework/` (CWD) | `crates/`, `docs/`, `agents/`, `skills/`, `prompts/`, CI, releases | NO | NO |
| **Bundle runtime** | `~/.local/share/sddk/framework/<v>/` | Snapshot: `agents/`, `skills/`, `prompts/`, `workflows/`, `assets/` | — | SÍ → `$HOME/.config/{opencode,claude,kilo,codex}/` |
| **Workspace de uso** | Repos del usuario | Proyecto + opcional `.sddk-versions` | SÍ | NO |

### 3.2. Resolución de versión (lookup en orden)

1. `$PWD/.sddk-versions`
2. `.sddk-versions` en directorios padre hasta raíz
3. `$SDDK_DATA_DIR/framework/current` (symlink global)

Formato (gestionado por el desarrollador, NUNCA por el framework):
```text
sddk 1.5.3
sddk current         # sigue el symlink global
sddk path:../..      # dogfooding (CWD = sddk-framework)
sddk system          # instalación del sistema
```
### 3.3. Cero intrusión

| Operación | Antes (mal) | Ahora (bien) |
|-----------|------------|--------------|
| Adopción | `workflow/workflow.yaml` plantado en repo | receipt en `~/.local/share/sddk/projects/<id>/` |
| Artefactos de ciclo | `sddk/{change}/...` en repo | `~/.local/share/sddk/projects/<id>/cycle-artifacts/{cycle_id}/` |
| Docs generados | `docs/generated/` en repo | `~/.local/share/sddk/projects/<id>/generated/` (o `--in-repo` para dogfooding) |
| Telemetry | `~/.local/share/sddk/uat-results.sqlite` | siempre XDG, nunca en repo |

---

## 4. Reglas de oro

### 4.1. Trabajar SIEMPRE desde el CWD (`sddk-framework/`)

- ✅ `cd ~/Proyectos/agentesIA/sddk-framework && git … && cargo …`
- ❌ `cd ~/.sddk-shared/ && …` — viola la regla "single source of truth en el CWD".
  **No crear nuevos checkouts en `~/.sddk-shared/`.**

### 4.2. El bundle runtime vive en `~/.local/share/sddk/framework/<v>/`

- Se actualiza con `sddk dev install` (o `sddk dev update`).
- **No es un checkout de git.** Es un snapshot publicado.
- **No edites directamente `~/.local/share/sddk/...`** — se sobrescribe en el próximo install.

### 4.3. El bundle runtime NO es un checkout del repo

- `agents/`, `skills/`, `prompts/` son **copias**, no symlinks. `bootstrap.sh`
  los symlinkea a los directorios de cada editor.

### 4.4. Las decisiones de diseño viven en `docs/adr/` o `~/.sddk-knowledge/`

- `docs/adr/` (este repo) — ADRs del proyecto público.
- `~/.sddk-knowledge/<project>/adrs/` — ADRs de proyectos adoptados.
- Specs del plan en `~/.sddk-knowledge/<project>/specs/`.

---

## 5. Checklist antes de commitear

```text
[ ] cargo build --release -p sddk-cli            # compila
[ ] cargo test --workspace                       # verde
[ ] cargo clippy --workspace                    # 0 errores
[ ] Si tocaste assets/: sddk dev install        # bundle runtime actualizado
[ ] git status                                  # clean
[ ] git diff                                    # revisas lo que vas a commitear
[ ] commit mensaje: feat(uat): … o fix(uat): …
[ ] git push origin main                        # pusheas
```

---

## 6. Resumen en una línea

> **El proyecto es el CWD** (`sddk-framework/`). El bundle runtime vive
> en `~/.local/share/sddk/framework/<v>/` (instalado por `sddk dev install`).
> Todo cambio de código va al CWD; todo cambio de contenido publicable se
> copia al bundle con `sddk dev install`.

---

## 7. See also

- **Historial de regresiones resueltas:** `docs/history/AGENTS-history.md`
- **Estado actual del proyecto (handoff):** `docs/handoff/HANDOFF-2026-08-13-sddk-framework.md`
- ** Roadmap de arquitectura:** `docs/sddk-2.0-architecture-consolidation/roadmap/ROADMAP.md`
