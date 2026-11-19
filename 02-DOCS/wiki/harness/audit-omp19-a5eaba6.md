# Aplicación de la auditoría rsc-harness / OMP 19

Estado: aplicado y verificado en local (2026-11-18). Cierra el artefacto
`/home/juan/Descargas/rsc-harness-auditoria-omp19-a5eaba6`.

## Intención

Incorporar al fork el contenido final de la auditoría OMP 19 sin arrastrar su
historial: el patch documenta el árbol resultado y el bundle conserva commits y
ancestros de upstream. Se eligió el patch para no mezclar 37 commits de upstream
en `main`; el bundle queda disponible sin aplicar.

## Alcance

Dentro:

- `rsc-harness`: patch aplicado sobre `a5eaba6` y verificado contra el árbol
  auditado; commit único en `main`.
- `speakerbox-acoustics`: `sync --target omp` con la entrada explícita del fork y
  registro de la distribución en `.rsc.json`.
- Este documento y su enlace en [../index.md](../index.md).

Fuera:

- Importar el bundle o los ancestros de upstream.
- Suite completa del fork y smoke de runtime OMP con modelos reales.
- Validación de Orca y `ovnfs` con sus runtimes.

## Checklist

- [x] `sha256sum -c SHA256SUMS` sobre patch y bundle.
- [x] `git apply --index --check` sobre `a5eaba6` (árbol limpio).
- [x] `git write-tree` devuelve `ec1f8f89a5825d0a835b5439acdd25461ac36825`, el árbol auditado.
- [x] Pruebas de los 4 ficheros tocados.
- [x] `manifest:check`, `validate`, `drift:check` y `originality:check`.
- [x] Commit en `main` del fork con gitmoji y Conventional Commits.
- [x] `provenance`, `sync --target omp` y `doctor --target omp` desde este proyecto.
- [x] Commit del resultado del sync en este repositorio.

## Evidencia

| Control | Resultado |
| --- | --- |
| Checksums | patch `3c2715b…` y bundle `a5ff83e…`: OK |
| `git write-tree` tras aplicar | `ec1f8f89…` = árbol de la auditoría |
| Commit del fork | `8a3c1c2` — `🔒 fix(distribution): verify CLI provenance and OMP 19 guard output` (15 ficheros, +277/−68) |
| Pruebas tocadas (`local-cli-instructions`, `omp-corrective`, `omp-native`, `rsc-cli`) | 43/43 |
| `manifest:check` | OK: 273 skills, 34 agentes, 53 comandos |
| `validate` | OK |
| `drift:check` | PASS: 988 enlaces, 4 no-claims |
| `originality:check` | OK: 82 documentos, 0 exenciones |
| `provenance` en este proyecto | `active` = `github:JhonMA82/rsc-harness`, 4.2.0; `pathCandidate: null`; `compatible: true` |
| `sync --target omp` | EXIT 0; 35 skills; instrucciones a `node '/home/juan/dev/rsc-harness/scripts/rsc.js' …` |
| `doctor --target omp` | `healthy: true`; `declared: github:JhonMA82/rsc-harness`; sin agentes ni comandos ausentes |
| Commit del sync | `4a55daa` — `🔧 chore(harness): sync OMP target from the fork CLI` (4 ficheros) |

El primer intento de commit del sync fue rechazado por el guard del proyecto:
sin gitmoji, y la cadena completa se bloquea, incluido el `git add`. Se repitió
add y commit en un solo comando con `🔧`.

## Límites conocidos

- La suite completa del fork no se ejecutó aquí. La auditoría reporta 1.954
  aprobadas, 2 omitidas y 1 fallida: A5 de Chromium, ambiental, más la
  canonicalización de `/var/tmp`, que se resuelve con
  `RSC_TEST_TMPDIR="$(realpath /var/tmp)"`.
- No se arrancó OMP con `-p`: gastaría llamadas reales al modelo.
- `doctor` avisa de que no hay `rsc` en PATH. Es esperado: las instrucciones
  generadas usan la entrada explícita del fork y no dependen del PATH.

## Siguiente paso

Ejecución E2E en TUI sobre un proyecto equipado: revisión nativa con
`xd://propose` hasta aprobación humana, workers de `task`, compactación y
reanudación con almacenamiento nativo estable. Si se quiere el historial de la
auditoría en `main`, importar el bundle en una rama revisada antes de mergear.
