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

### 5.1. `~/.sddk-shared/` es un segundo checkout del repo (DRIFT)

```
$ ls -d /var/home/rubentxu/.sddk-shared
/var/home/rubentxu/.sddk-shared    # existe

$ git -C /var/home/rubentxu/.sddk-shared remote -v
origin  https://github.com/Rubentxu/sddk-framework.git
```

**Problema**: por el spec RS-2026-08, no debería existir un segundo
checkout del repo. Todo el trabajo de desarrollo debe ocurrir en el CWD
(`sddk-framework/`).

**Acción**: cuando el usuario confirme, eliminar `~/.sddk-shared/` con
`rm -rf`. Antes de hacerlo:
1. Verificar que no hay commits uncommitted en ese checkout.
2. Verificar que ningún editor está apuntando a `~/.sddk-shared/`
   (buscar en `~/.config/opencode/`, `~/.claude/`, etc.).
3. Actualizar `bootstrap.sh` (ver §5.2) ANTES de eliminar el dir.

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
> `~/.sddk-shared/` es un segundo checkout **a eliminar** (drift del
> modelo asdf-vm). Todo cambio de código va al CWD; todo cambio de
> contenido publicable se copia al bundle con `sddk dev install`.
