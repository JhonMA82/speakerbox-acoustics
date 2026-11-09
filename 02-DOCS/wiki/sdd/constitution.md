# Constitución — speakerbox-acoustics

Ratifica los no negociables ya vigentes en `AGENTS.md` y `docs/AGENTS.md`:

1. API público en SI; `from_ui_units` acepta cm², cm, L, g, mH, mm.
2. `xd_mm` es desplazamiento de pico (`√2`·RMS) en todas las topologías.
3. `phase_deg` es fase de velocidad volumétrica; la presión añade +90°.
4. `solid_angle` en estereorradianes (`4π`, `2π`, `π`, `π/2`).
5. La calibración es empírica y ligada a la geometría validada: no se cambia sin
   revalidar los tests de producción.
6. No se añade una topología sin export propio y test puntual.
7. El núcleo no depende de GUI, egui, SpeakerBox ni de una aplicación consumidora.
8. MSRV 1.75; sin dependencias nuevas sin necesidad demostrable.
9. `cargo test --workspace` verde y métricas de producción sin degradación.

Estado: ratificada provisionalmente para esta ejecución (autopilot).
