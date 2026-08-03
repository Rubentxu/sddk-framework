# SDDK Framework

> **Spec-Driven Development Kernel** — an agentic software engineering workflow with a built-in knowledge graph, trunk-based git discipline, and multi-lens verification.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![OKF Compatible](https://img.shields.io/badge/OKF-v0.2-blue.svg)](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
[![Obsidian Compatible](https://img.shields.io/badge/Obsidian-Properties_v1.4+-purple.svg)](https://obsidian.md/)

**[English](README.md)** | [Español](README.es.md)

---

## What is SDDK?

SDDK is a complete agent orchestration framework for AI-assisted software development. It coordinates AI agents through a structured pipeline — from exploration to release — with built-in quality gates, technical debt auditing, and a knowledge graph that tracks every decision, requirement, and incidence across cycles.

### Key differentiators

| Feature | What it does |
|---------|-------------|
| **Spec-Driven** | Every change starts with a spec (Given/When/Then scenarios). Implementation is verified against the spec, not just "does it compile." |
| **Multi-lens verification** | 6 parallel verification lenses (spec compliance, architecture, test quality, design coherence, 2 adversarial judges) + synthesis. |
| **Technical debt audit** | 5 cluster agents (architecture, smells, duplication, coupling, over-engineering) audit debt before merge to main. |
| **Knowledge graph** | Every milestone, ADR, requirement, cycle, and incidence is a node in an Obsidian-compatible wikilink graph. Full bidirectional traceability. |
| **Trunk-based guarantee** | A cycle cannot declare `success` until changes are merged to `main` + semver tagged + trunk synced. No silent aborts. |
| **Serialization lock** | Only one cycle at a time. The lock survives session crashes. |
| **Editor-agnostic** | Works with ZCode and OpenCode (extensible to any agent runner). |

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   ~/.sddk-shared/                    │
│              (this repository — framework)           │
│                                                      │
│  ┌──────────┐  ┌─────────┐  ┌────────────────────┐  │
│  │  agents/  │  │ skills/ │  │ prompts/sdd-kernel │  │
│  │  (63)     │  │ (89)    │  │ (phase specs, MCW) │  │
│  └────┬─────┘  └────┬────┘  └─────────┬──────────┘  │
│       │              │                  │             │
│  ┌────┴──────────────┴──────────────────┴──────────┐ │
│  │         knowledge-template/ (vault template)     │ │
│  │  milestones · adrs · specs · cycles · incidences │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─────────────┐  ┌──────────────────┐               │
│  │golden-dataset│  │ bootstrap.sh     │               │
│  │(meta-testing)│  │ (installer)      │               │
│  └─────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────┘
         │                                    │
    ┌────┴────┐                         ┌─────┴─────┐
    │ ZCode   │                         │ OpenCode  │
    │(symlinks)│                        │(symlinks) │
    └─────────┘                         └───────────┘
         │
    ┌────┴──────────────┐
    │ ~/.sddk-knowledge/{project}/ │  (per-project vault,
    │       (committed to git)    │   created by sddk-adopt)
    └───────────────────┘
```

## Quick start

### Install

```bash
git clone https://github.com/Rubentxu/sddk-framework.git ~/.sddk-shared
~/.sddk-shared/bootstrap.sh --all
```

The bootstrap script auto-detects installed editors (ZCode, OpenCode) and creates symlinks. Your project repos stay clean — **zero documentation files in your code repos**.

### Run a cycle

**First time on a project?** Adopt it first:

```bash
cd your-project
/sddk-adopt         # one-time: audit project, plant SDDK artifacts, create knowledge vault
/sddk-init          # one-time: detect stack, testing, TDD mode
/sddk-new add-auth  # start a full SDDK cycle
```

**Subsequent cycles** (project already adopted):

```bash
cd your-project
/sddk-new <change-name>  # the ~/.sddk-knowledge/{project}/ vault is already there; init is skipped
```

The `~/.sddk-knowledge/{project}/` directory is the adoption marker — its existence means the project is adopted. `sddk-init` checks it with a single `test -d`.

The orchestrator will:
1. **Plan** — explore → propose → spec → design → tasks (interactive checkpoints)
2. **Build** — apply (with Strict TDD if enabled) → verify (multi-lens) → debt-verify (5 clusters)
3. **Release** — push → PR → merge to main → semver tag → update knowledge graph → sync trunk

No cycle closes until your code is on `main`.

## Workflow paths

| Path | When | Depth |
|------|------|-------|
| **B-direct** | Hotfix, bounded task | Load skill → execute → light verify → release |
| **A-min** | Simple change, C2 context | spec → apply → verify → debt-verify (smoke, 2 clusters) → release |
| **A-lite** | Bounded work, C1 context | propose → spec → apply → verify → debt-verify (standard, 4 clusters) → release |
| **A-full** | Architectural, new domain, C0 | explore → propose → spec ∥ design → tasks → apply → verify (6 lenses) → debt-verify (deep, 5 clusters) → release |

Reversibility axis (v3.4) modulates debt-verify depth independently:
- **High reversibility** (pure code, feature-flagged) → skip debt-verify
- **Low reversibility** (schema, security) → force deep + judgment-day

## Knowledge graph

Every cycle populates a knowledge vault at `~/.sddk-knowledge/{project}/` (in user home, outside the repo):

```
my-app/~/.sddk-knowledge/{project}/
├── _index.md              ← MOC with Dataview queries
├── milestones/
│   ├── _active.md         ← serialization lock
│   └── M-001-auth.md      ← [[ADR-003]], [[REQ-Session]]
├── adrs/
│   └── ADR-003-jwt.md     ← [[REQ-Session]], implementation log
├── specs/auth/
│   └── REQ-Session.md     ← [[ADR-003]], tested_by, verified_in_cycle
├── cycles/
│   └── CYC-2026-08-03.md  ← traceability hub (links everything)
├── incidences/
│   └── INC-001-lag.md     ← [[ADR-003]], affects [[REQ-Session]]
└── terms/
    └── TERM-JWT.md
```

Open it in [Obsidian](https://obsidian.md) for graph view, backlinks, and Dataview queries. Based on [Google's OKF spec](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) with bi-temporal changelogs.

## Verification system

### Functional verification (`sddk-verify`)

The **Behavioral Compliance Matrix** maps every spec scenario to a test that passed at runtime. Static analysis alone is never verification.

| Lens | What it checks |
|------|---------------|
| Spec Compliance | Every scenario → covering test → runtime PASS |
| Architecture + Connascence | Design quality, coupling, SOLID |
| Test Quality | Banned assertions, mock ratios, triangulation |
| Design Coherence | Design decisions vs implementation |
| Adversarial Judge A | Blind deficiency detection |
| Adversarial Judge B | Blind deficiency detection |

### Technical debt audit (`sddk-debt-verify`)

5 cluster agents run in parallel (read-only on codebase):

| Cluster | Dimension |
|---------|-----------|
| Architecture | Connascence, SOLID, Matsumoto + Khononov critiques |
| Smells | 12 Fowler smells with grep-verifiable signals → SOLID mapping |
| Duplication | Structural/literal/semantic + dead code |
| Coupling | Hidden deps, global state, circular imports |
| Over-engineering | YAGNI, ponytail debt ledger, bloat trajectory |

## Project structure

```
sddk-framework/
├── agents/                 # 63 agent prompts (orchestrator, phase executors, clusters, judges)
├── skills/                 # 89 skills (knowledge-graph, sddk-*, entropy-sdd, cognicode-sdd, ...)
├── prompts/sdd-kernel/     # Phase specs, MCW, git-contract, decision-model, ADR/roadmap templates
├── knowledge-template/     # Vault template (6 node types, MOCs, serialization lock)
├── golden-dataset/         # Meta-verification test cases (5 initial cases + runner)
├── bootstrap.sh            # Installer for ZCode/OpenCode
├── README.md               # This file
├── README.es.md            # Spanish documentation
└── LICENSE                 # MIT
```

## Key concepts

- **MCW (Mandatory Complete Workflow)** — the law. 5 phases, numbered steps, hard gates. Source of truth: `prompts/sdd-kernel/mcw.md`.
- **Serialization Lock** — one cycle at a time. Lock file: `milestones/_active.md`. Survives session crashes.
- **Release Completion Guard** — the orchestrator cannot emit `status: success` without `HEAD == origin/main` + semver tag confirmed on remote.
- **Zero docs in repo** — all project knowledge lives in the vault, never in the project's git repo.
- **Bi-temporal changelog** — every node tracks `valid_from` / `valid_to`, enabling time-travel queries.

## Compatibility

- **Editors**: ZCode, OpenCode (extensible to any agent runner that reads markdown prompts)
- **Knowledge format**: [OKF v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md), [Obsidian Properties v1.4+](https://obsidian.md/)
- **MCPs** (optional): CogniCode (architecture analysis), Chronos (time-travel debugging), LogSeq (alternative vault), Engram (cross-session memory)

## Contributing

Contributions are welcome. Please read the architecture in `prompts/sdd-kernel/mcw.md` before proposing changes.

## License

[MIT](LICENSE) © 2026 Rubentxu
