# Decisiones

- 2026-11-09 — Rama `stabilization/library-audit` en lugar de desbloquear
  `main`; `main` está cerrado por CI. Decisión del usuario.
- 2026-11-09 — `parse_tsv`/`parse_csv` pasan a `Result`; es un cambio
  incompatible necesario para no perder columnas ni aceptar celdas inválidas.
  Justifica el bump `0.2.0`.
- 2026-11-09 — `ze_phase_deg` no se compara en `validate`: no hay contrato ni
  tolerancia validada; se documenta la limitación.
- 2026-11-09 — Se elimina el fallback del 5 % de Mms: un `Mms` menor o igual
  que la masa de aire es físicamente inconsistente y se rechaza.
