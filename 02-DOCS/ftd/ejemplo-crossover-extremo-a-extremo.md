# Ejemplo de extremo a extremo: crossover cargado por la caja

Estado: implementado y verificado en local (rama `main`, 2026-11-18).

## Intención

Dar una prueba reproducible de la cadena que hoy solo existía descrita en
[Arquitectura](../../docs/ARCHITECTURE.md): la impedancia de bornes que calcula el
motor acústico termina el filtro pasivo de `speakerbox-circuits`, y la
transferencia compleja del análisis nodal escala la presión de la caja.

El ejemplo responde a una pregunta concreta: ¿cuánto se desvía un crossover
diseñado con la impedancia nominal cuando se carga con el driver real?

## Alcance

Dentro:

- `examples/crossover_end_to_end.rs`: caja ventilada de 50 L, prototipo
  Butterworth LP2 a 2 kHz para 6 Ω, MNA con carga constante y con la curva de
  impedancia real, y SPL filtrado.
- `speakerbox-circuits` como dependencia de desarrollo de la raíz. Ninguna
  biblioteca publicada cambia de dependencias.
- Corrección del párrafo de dependencias en `docs/ARCHITECTURE.md` y entrada
  en `CHANGELOG.md`.

Fuera:

- Rediseñar el filtro contra la carga real, medir un driver físico o validar
  contra Hornresp. El ejemplo no crea oráculos ni altera calibraciones.
- Tratar la impedancia de salida del amplificador: pertenece a `System::rg`,
  no al filtro.

## Checklist

- [x] Manifiesto raíz con `speakerbox-circuits` como `dev-dependency`.
- [x] Ejemplo compilable con la API pública de ambas crates.
- [x] MNA verificado contra el prototipo analítico del propio crate.
- [x] `cargo fmt --all` sin diferencias pendientes.
- [x] `cargo check --workspace --all-targets`.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] `cargo test --workspace` (todas las suites en verde).
- [x] `cargo run --example crossover_end_to_end`.
- [x] `cargo +1.75.0 check --workspace --all-targets` con el toolchain 1.75.0.
- [x] `cargo check --workspace --lib --target wasm32-unknown-unknown`.

## Evidencia

Salida real de `cargo run --example crossover_end_to_end` (extracto):

```text
Error máximo MNA vs analítico: 2.89e-15 dB

   f/Hz    |Z|/Ω    ∠Z/°   |H|real     |H|6Ω    Δ/dB  SPL caja SPL filtro
   1000     8.48    44.8     -1.27     -0.26   -1.01     85.11      83.84
   1500    11.00    56.9      0.22     -1.19    1.42     79.96      80.18
   2000    13.78    64.2      4.21     -3.01    7.22     74.87      79.08
   3000    19.68    72.2      3.56     -7.83   11.38     68.64      72.19
```

Lectura:

- El MNA reproduce el prototipo Butterworth con error de máquina; no hay tabla
  heredada ni ajuste empírico.
- A 2 kHz el driver presenta `13,78 Ω ∠ 64,2°`, no 6 Ω. El condensador en
  derivación resuena con la inductancia de bobina (1 mH) cerca de 1,64 kHz y la
  red entrega `+4,2 dB` donde el prototipo prometía `-3,0 dB`. Error de 7,2 dB
  en el punto de cruce.

## Siguiente paso

Todas las puertas del [archivo raíz](../../AGENTS.md) pasan en local. Si el
ejemplo debe crecer, el siguiente paso es una segunda pasada que rediseñe el
filtro contra `Z(f)` y compare el punto de cruce resultante con el de 6 Ω.
