# Evidencia y comparación reproducible

La unidad de evidencia es una afirmación sobre un observable, bajo una geometría,
una banda de frecuencias, una excitación y una condición de contorno. Un mismo
resultado puede tener varios tipos de evidencia con alcances distintos.
Los límites cuantitativos corresponden a cada fila.

| Tipo de evidencia | Alcance | Criterio y resultado |
| --- | --- | --- |
| Verificación analítica | Geometría circular de tractrix, tractrix escalada, OS y unión exponencial-cónica. Extremos, puntos, tangencia, integración de volumen y subdivisión. | Comparación contra la construcción de tangente constante y fórmulas cerradas. Pruebas de convergencia de volumen en `tests/horn_profile.rs`. |
| Verificación analítica | Tubo de aire uniforme de radio 25 mm y longitud 270 mm, 25–630 Hz, capa delgada, fasores RMS. Matriz y atenuación. | Comparación con Kirchhoff de sección circular, evaluado con Bessel complejas. Error máximo observado 0.00272 dB, 0.0394° y 0.930 % en atenuación. Al partir el tramo, las matrices coinciden dentro de 3·10⁻¹² relativo. |
| Comparación numérica independiente | Webster RK4 Rust frente a TMM de cilindros del otro proyecto. Once condiciones geométricas y de contorno, 25–630 Hz. Impedancia de entrada, presión y caudal de boca, disipación interna. | 99 puntos. Umbrales 0.015 dB y 0.1° para cada fasor, y 10⁻⁵ de potencia de entrada para pérdidas y conservación. Máximos 0.000164 dB, 0.00220° y 3.41·10⁻¹² para pérdidas. |
| Comparación numérica independiente | Bocina cónica con motor sintético, parte trasera cerrada, 25–630 Hz. Conexión ideal, cámara concentrada y adaptador cilíndrico. | 27 puntos. Ze, Za, movimiento, presión/caudal de garganta y caudal de boca a 0.015 dB y 0.1°. SPL de potencia y fase reducida con los mismos umbrales. Excursión y potencias a 2·10⁻⁴ relativo. Residual relativo de potencia ≤ 3.22·10⁻¹². |
| Comparación con software de referencia | Hornresp 60.70, con geometrías, topologías, observables y tolerancias ya registrados por el proyecto. | Métricas observadas conservadas. [Oráculos de producción](../VALIDATION.md) y [comparación antes/después](RESULTADOS.md). |
| Validación experimental mediante mediciones | En la referencia externa, impedancia de un horn exponencial y subconjunto de tubos. | Evidencia de su backend y sus ensayos. Se mantienen visibles el fallo de reflectancia cónica y los 62 casos de tubo que no pasan. [Resultados externos](https://github.com/timini/horn-simulation/blob/7dd9852aec8f60e1b35ae637ad28bd614efc2c1b/docs/VALIDATION.md). |
| Predicción experimental sin validación física suficiente | Adaptador frontal y pérdidas termoviscosas de Rust en geometrías reales, modos transversales, respuesta angular y candidatos FEM. | El adaptador y las pérdidas son optativos. Sus garantías actuales son analíticas y numéricas para las condiciones anteriores. Quedan pendientes mediciones con geometría y excitación documentadas. |

## Condiciones del fixture

`tests/data/horn-simulation/webster.json` conserva la revisión de origen,
versiones de NumPy y SciPy, aire, excitación, cuadrícula, perfiles, fronteras,
pérdidas y series complejas como pares real/imaginario. Cada magnitud se
evalúa en todos los puntos. Las longitudes de tractrix natural admiten
diferencias de redondeo de los constructores. Al regenerar, se comprueba que
duplicar los cilindros de 2,048 a 4,096 cambia menos de 5·10⁻⁵ en módulo
relativo complejo para las tres salidas de transferencia. En Rust,
`tests/horn_simulation_reference.rs` exige los casos y sus condiciones
esperadas. Los casos negativos incluyen un fixture vacío, valores ausentes,
impedancia cero incompatible, signo de fase o norma de amplitud incorrectos,
unidad incorrecta y frontera de boca distinta.

- Aire de la referencia `c=343 m/s`, `ρ=1.225 kg/m³`, `p_ref=20 µPa`.
  Para termoviscosidad, `μ=1.81·10⁻⁵ Pa·s`, `κ=0.0257 W/(m·K)`,
  `Cp=1,006 J/(kg·K)` y `γ=1.4`.
- TMM a 1 Pa RMS en garganta, convención `exp(+jωt)` y caudal positivo de
  entrada a salida. El TMM reporta impedancia específica `p/v`. Se divide por
  el área física de entrada para comparar impedancias en `Pa·s/m³`.
- La boca se termina con pistón en pantalla infinita a 2π sin calibración, o
  con `ρc/S_boca` en el caso adaptado. La transferencia es presión media de
  sección y caudal por Pa de presión total impuesta en garganta.
  Para el motor se usa una tensión de 2.83 V RMS. El test usa SPL de potencia a 1 m y fase reducida de la librería.
  La presión Rayleigh en eje de la referencia es un observable distinto.
- La energía activa usa `Re(p·conj(U))`, con excitaciones RMS. La excursión
  usa `√2·|v_d|/ω`. La potencia de pérdidas de Rust se integra en el conducto
  y se compara con la diferencia de flujos del TMM.
- En la geometría tractrix escalada, la referencia aproxima por interpolación
  lineal una curva que Rust evalúa como contorno continuo. Se conserva esta
  diferencia geométrica en la comparación. El modelo Kirchhoff completo se
  usa como verificación del modelo delgado en el tubo, con el error de la
  aproximación explícito en la tabla.
- Se excluyen cruces Darcy frente a termoviscoso, Robin FEM frente a
  concentración 1D y campo Rayleigh frente a SPL de potencia. La coincidencia
  en uno de esos observables no sería evidencia de equivalencia física.

## Reproducción

Ejecutar la comparación con el fixture incluido requiere solo Cargo.
Para rehacerlo, prepare un checkout limpio de la referencia, instale las
mismas versiones de NumPy y SciPy en un entorno Python y ejecútelo desde la
raíz de `speakerbox-acoustics`.

```bash
cargo test --locked --test horn_simulation_reference -- --nocapture
git clone https://github.com/timini/horn-simulation.git ../horn-simulation
git -C ../horn-simulation checkout --detach 7dd9852aec8f60e1b35ae637ad28bd614efc2c1b
python3 -m venv .venv
. .venv/bin/activate
python -m pip install numpy==2.3.5 scipy==1.17.0
python tools/cross-validation/horn-simulation/generate.py ../horn-simulation /tmp/horn-webster.json
HORN_SIMULATION_FIXTURE=/tmp/horn-webster.json cargo test --locked --test horn_simulation_reference -- --nocapture
```
