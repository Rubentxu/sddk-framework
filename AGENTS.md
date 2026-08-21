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

## 4. Reglas de oro

- **CWD es el repo** (`~/Proyectos/agentesIA/sddk-framework/`). Nunca `~/.sddk-shared/`.
- **Bundle runtime** (`~/.local/share/sddk/framework/<v>/`) es un snapshot publicado,
  no un checkout git — no editarlo directamente.
- **`agents/`, `skills/`, `prompts/` son copias** (no symlinks). `bootstrap.sh` los
  symlinkea a los directorios de cada editor.
- **Decisiones de diseño** en `docs/adr/` (repo) o `~/.sddk-knowledge/<project>/adrs/`.

---

## 5. Checklist antes de commitear
`cargo build --release -p sddk-cli && cargo test --workspace && cargo clippy --workspace && cargo fmt --all -- --check && git commit -m "…" && git push origin main`

---

## 6. Resumen en una línea

> **El proyecto es el CWD** (`sddk-framework/`). El bundle runtime vive
> en `~/.local/share/sddk/framework/<v>/` (instalado por `sddk dev install`).
> Todo cambio de código va al CWD; todo cambio de contenido publicable se
> copia al bundle con `sddk dev install`.

---

## 7. See also

- **Release & distribution:** `docs/RELEASING.md`
- **Architecture model:** `docs/ARCHITECTURE-MODEL.md`
- **Historial de regresiones resueltas:** `docs/history/AGENTS-history.md`
- **Estado actual del proyecto (handoff):** `docs/handoff/HANDOFF-2026-08-13-sddk-framework.md`
- ** Roadmap de arquitectura:** `docs/sddk-2.0-architecture-consolidation/roadmap/ROADMAP.md`
