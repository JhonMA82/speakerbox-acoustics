# AGENTS.md

Guía rápida para agentes que trabajen en `speakerbox-acoustics`. La versión
detallada está en [`docs/AGENTS.md`](docs/AGENTS.md); la guía de integración
para aplicaciones está en [`docs/INTEGRATION.md`](docs/INTEGRATION.md).

## Estado

**v1 cerrado.** Topologías validadas a producción contra exports propios de
Hornresp 60.70:

- Bocina normal.
- OD / OD1 / OD2.
- TH (S2/S4) y TH1 (S2/S3).
- BP4, BP6 paralelo y BP6 serie.
- Caja sellada, ventilada y radiador pasivo.

No se aceptan topologías nuevas sin modelo, oráculo propio y test de producción
puntual.

**Fase 3.1 en curso:** `DriverBank` implementado para N drivers, paralelo/serie/
serie-paralelo, isobaric, push-pull y DVC paralelo/serie. Pendiente:
`RadiatorArray` para line arrays y validación de producción con exports propios
multi-driver.

## Comandos obligatorios

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo check --lib --target wasm32-unknown-unknown
cargo +1.75.0 check --workspace --all-targets
```

Métricas por topología:

```bash
cargo test --test od_production -- --nocapture
cargo test --test th_production -- --nocapture
cargo test --test bp_production -- --nocapture
cargo test --test system_production -- --nocapture
cargo test --test production_accuracy -- --nocapture
```

## Invariantes

- API público en **SI**; los `from_ui_units` aceptan cm², cm, L, g, mH, mm.
- `xd_mm` es desplazamiento de **pico** (`√2`·RMS) en todas las topologías.
- `phase_deg` es fase de presión reducida por j (en OD, suma total); la
  presión física añade +90°.
- `solid_angle` en estereorradianes (`4π`, `2π`, `π`, `π/2`).
- `response_at` devuelve un punto neutro; `try_response_at` devuelve
  `SolveError`.
- La calibración es empírica y está ligada a la geometría validada. No cambie
  constantes de calibración sin revalidar los tests de producción.
- No añada una topología sin export propio de Hornresp y test puntual.
- No copie datos ni código de proyectos con licencia incompatible.

## Mapa mínimo

```text
src/system.rs      System: radiador directo, caja, bocina, puerto, PR
src/driver_bank.rs bancos de drivers (N, wiring, isobaric, DVC)
src/driver_import.rs importación .wdr / .owdr
src/tapped.rs      TappedHorn: TH/TH1
src/offset.rs      OffsetHorn: OD/OD1/OD2
src/bandpass.rs    Bandpass: BP4/BP6P/BP6S
src/design.rs      Design/DesignPoint para la UI
src/validation.rs  OraclePoint, validate, tolerancias
tests/data/        registros y oráculos propios
docs/              integración, algoritmos y guía detallada de agentes
```

## Regla de oro

Cada cambio debe dejar `cargo test --workspace` en verde y no debe degradar las
métricas de producción. Si una topología no tiene oráculo propio, queda como
experimental y no entra en v1.

## Auditoría física correctiva

Consultar `docs/audits/ACOUSTIC_AUDIT.md` y sus cuatro informes asociados.
SPL/Ze/Xd siguen pasando sus oráculos; potencia/rendimiento legados son
estimaciones con limitaciones medidas, no un balance energético general.
No cambiar calibración ni ampliar las garantías sin resolver ese modelo.

## Segunda pasada energética

Estado vigente: docs/audits/RADIATION_MODEL_CONTRACT.md. Los campos legados
power_w/acoustical_power_w son absorción de cargas explícitas (pueden ser
subtotales), no potencia de campo. try_energy_at conserva calidad radiativa,
fuente/bornes/pérdidas y eficiencias opcionales. Matched no define SPL; fase
legada NaN en silencio y eficiencia NaN para 0/0 son ausencias deliberadas.
Conservar calibración, oráculos y optimizaciones de la primera pasada.


<!-- rsc-suggest:start -->

# rsc-suggest — the always-on layer

Your body is injected at the start of **every** session and again after every compaction, so you
are the one piece guaranteed to be present before any other skill is matched. Two jobs, in order:

1. **Classify every turn into one of three lanes** before anything is written.
2. **Keep the session equipped** — spot the skill the task needs but the user does not have.

---

## 1. The decisor: classify the turn before acting

Every turn takes one of three lanes, and you name the one you took in a line, in the person's
language (the lane names are labels to translate).

**Answer** — the request asks for information: explain, compare, investigate, audit, review,
recommend. Read-only: write nothing, create no artifact, delegate no writer. Asking you to *think
about* building something is still this lane; only asking to build authorises it. When change
intent is ambiguous, ask one question and stay here — ambiguity slows the lane, never raises it.

**You choose the lane; never hand that choice to the person.** Simple → **FTD**: one feature document
in `02-DOCS` (intent, scope, checklist, evidence, next step), tasks checked off against observed proof.
Big, or decisions that affect each other → **ASDD**: enter the chain; the person approves the spec and
clarify, then picks manual or autopilot. Say which and why in one line; switch if asked.
→ `../ftd/SKILL.md` · `../asdd/SKILL.md`.

**Where.** Default branch open (chosen at install, or nothing complex) → work on it, no branch
question. Closed («ramas y PR», CI, team) → before each code change ask: this branch, a new one, or
`rsc main unlock`? Never branch alone. Another session here → `.worktrees/<branch>`.

Judge the **meaning**, not the wording: the trigger is semantic in any language. A bug fix restoring
intended behaviour is `debug`. Autopilot consent covers a whole run — advance without re-asking.

Lane's skill missing? Offer it (§2) first. ASDD recorded as deferred in `.rsc.json`? Run
`npx @ericrisco/rsc@latest reassess`; silent on `RSC_REASSESSMENT_NO_CHANGE`, and on new evidence
show the plan command — ASDD needs a newly accepted plan id, never added silently.

---

## 2. Keeping the session equipped

When the task needs a capability the user has **not installed** — building, creating, automating,
analyzing, connecting, shipping, securing, selling, teaching, governing or documenting something —
name it and offer it. This runs mid-conversation, not only at project start.

1. `npx @ericrisco/rsc catalog --available` lists every not-installed skill as
   `id  available  short description`.
2. Pick the single best fit **by meaning**, the way you would match a request to a teammate's
   expertise, even when no keyword overlaps. If nothing genuinely fits, say so and move on: a
   tangential suggestion is worse than none.
3. Ask once, plainly: "Para esto instalaría `<id>`, que aún no tienes. ¿La instalo? (sí/no)".
4. On yes, run `npx @ericrisco/rsc add <id>`, then continue the original task.

One suggestion at a time,
and never to interrupt a flow with a nice-to-have. Never recommend something already installed
(`npx @ericrisco/rsc list`).

`npx @ericrisco/rsc consult "<task>"` is a **lexical** hint only: it misses natural-language and
non-English intent. Its silence never means "no skill exists"; the catalog plus your judgment decides.

### Automation gap — after the work

Delivered work a repeatable **procedure**? Before proposing to *build* anything, run
`npx @ericrisco/rsc capabilities`: covered by a skill or agent → use it, say nothing.
Not covered → one line, skill or agent. Rules: `skill-scout`.

---

## 3. A harness that is broken fixes itself

Its owner rarely notices: the symptoms name nothing they know. When any of these
is true, act on it **once** in the session:

- `.rsc.json` exists and what it declares is not what is installed;
- the same hook seems to run several times;
- the harness is wired for an assistant that is not the one running.

Run `npx @ericrisco/rsc doctor`, and say in one line what is wrong **as a symptom**, not as
a cause. Then:

- **Nothing built** — `.rsc.json` without `.rsc/`: a clone. Name
  `npx @ericrisco/rsc@<catalogVersion in .rsc.json> sync`, that exact version, **never** `@latest`
  — a release nobody here adopted is drift. Ask in one line, **keep working either way**; silence
  is not a no, `.rsc/.no-harness` is.
- Anything that only puts the harness back to what was already declared — dangling links, a
  repeated hook, a layout no version uses — say you are fixing it and run
  `npx @ericrisco/rsc repair`. Restoring is not deciding, and a recoverable copy is kept.
- Anything that would change a decision — moving to another assistant, adopting a skill a
  teammate added, disarming a gate — **ask first**. A `git pull` never rewrites someone's
  machine.

Offer once per session; say nothing when the harness is healthy.

## 4. First contact

Before handling the first request of a session, check the workspace:

- No `02-DOCS/wiki/harness/user-profile.md` **and** no `.rsc/.no-harness` → invoke `init` first
  (one question: technical terms or analogies) before the task.
- A clone (`.rsc.json` committed) is not first contact: ask that once, in one line, then continue
  with their task; with `.rsc/.profile-offered`, never again.
- The user declines a harness here ("sin harness", "solo código") → create an empty
  `.rsc/.no-harness`, confirm in one line, and never auto-start `init` in this repo again.
- With a profile, this gate is inert. Never re-onboard.

## 5. «Recuerda esto»

A project or business fact goes to its topic page in `02-DOCS/wiki/` (link a new page from
`wiki/index.md`); say the path. Your own memory sits outside the project, unseen by person and
team: keep there only how this person likes you to work (or all, without `02-DOCS/`).

## Explain without assuming

Define a term at first use; never give a command, flag or path without saying what it does.

## Orientación (siempre)

Habla con la voz de `orient`: frases cortas, una idea por frase, al grano, y cada respuesta se entiende sola. Registro técnico o con analogías según `technical_level` en `02-DOCS/wiki/harness/user-profile.md`. Cierra cada turno con el **bloque-brújula** (📍 dónde estás · ➡️ siguiente, terminando en pregunta; ✅ y 🧭 cuando hay algo hecho o decidido). **Nunca termines en seco.** Protocolo completo: skill `orient` → `skills/orient/references/orientation-contract.md`. (Defiere a §2 el "¿instalo la skill que falta?".)

**rsc updates.** Claude Code, Codex, Gemini CLI, Cursor, OpenCode and Oh My Pi check for a new rsc on their own. In any other assistant, run `node .rsc/auto-update.mjs` once on the first turn of a session and relay any notice it prints in one line; "up to date" needs no mention.

<!-- rsc-suggest:end -->
