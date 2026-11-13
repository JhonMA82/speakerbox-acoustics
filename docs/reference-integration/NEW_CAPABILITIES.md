# Contratos de las capacidades nuevas

Esta página sustituye la descripción de la integración inicial parcial. Se
conserva `reference_design::uniform_transmission_line` para compatibilidad;
la topología completa es `TransmissionLine` y se integra mediante `Design::from`.
No se importa código ni datos de loudspeaker-sim, ni se añade dependencia de producción.

```rust
use speakerbox_acoustics::{Driver,CoilModel,Segment,Flare,TransmissionLine,Design,SolveError};
fn main() -> Result<(), SolveError> {
let driver=Driver::from_ui_units(350.,4e-4,20.,6.,18.,4.,1.,1)
    .with_coil_model(CoilModel::SemiInductance{ke:0.2});
let sections=vec![Segment::new(Flare::Cylindrical,1.,0.02,0.02)
    .with_flow_resistivity(1000.)];
let line=TransmissionLine::at_position(driver,sections,0.33)?;
let state=line.try_response_at(50.)?; // complejos RMS, excursión pico en mm
let energy=line.try_energy_at(50.)?;
let design=Design::from(line);
let sweep=design.try_sweep_log(20.,200.,100)?;
Ok(())
}
```

SI: longitudes m, áreas m², volúmenes m³, frecuencia Hz, Rf Pa·s/m²,
Ke Ω√s, impedancia acústica Pa·s/m³, masa de pliegue kg/m⁴, resistencia de
pliegue Pa·s/m³. TL usa longitud recorrida, no longitud proyectada. Cada
segmento define su zona de relleno; densidad del relleno no se convierte en Rf.
`tap` es un nodo; `at_position` divide el perfil en cualquier x entre 0 y L.
Los pliegues se colocan entre segmentos dentro de una rama, no en el nodo del
propio driver sin especificar de qué lado pertenecen.

`LinePoint` conserva presión/caudal de boca, presión de nodo, caudales de ramas,
caudal del driver, presión exterior estimada e impedancia eléctrica de entrada
(incluye Rg, como los puntos históricos). La fase de Design es la reducida por
j; la fase de `pressure_estimate_pa` es física y suma +90°. La presión supone
fuentes compactas, coplanares y la misma referencia de recorrido a 1 m. No
simula directividad espacial ni carga mutua. Una terminación resistiva artificial
es absorción, no una fuente exterior adicional. `EnergyReport` conserva
fuente, bornes, calor, disipación acústica y calidad radiativa separadas.

`Segment::try_matrix` devuelve Result con dominio numérico comprobado.
`matrix` conserva la firma infalible y usa NaN si no hay matriz interpretable;
no devuelve identidad ni cero como sustituto. Los consumidores científicos
utilizan APIs estrictas. No basta dividir un tramo para eludir la cota de
atenuación de la cascada.

## Diseño y optimización

`optimization::minimize(bounds,start,config,evaluator,cancel)` no modifica un
diseño. El callback crea una copia válida a partir de parámetros y devuelve
Some(coste) o None si el solver/geometría falla. `Bound::locked` fija parámetros;
los límites son restricciones duras. `acoustic_cost` permite límites duros y
penalizaciones por frecuencia: SPL, excursión, puerto y potencia de calidad
validada. Dos objetivos opuestos representan un target SPL; una banda de
frecuencias representa uniformidad/extensión. Volumen y dimensiones se
representan mediante parámetros y restricciones del callback. La semilla fija
DE; la cancelación se consulta antes de cada evaluación. El mejor candidato
válido se conserva aun con cancelación o presupuesto agotado. Ningún estado
`Converged` garantiza el mínimo global.

`alignments::sealed` y `vented` son síntesis ideales con sus dominios explícitos.
B4/Bessel/C4 requieren Qts compatible; no se inventa otro driver. QB3/SC4 tienen
dominios diferentes. `VentedAlignment::to_system` genera un sistema y puerto,
rechazando longitud negativa. `solver_deviation_db` mide el desajuste con el
ideal. EBS es una propuesta numérica con coste explícito, no una fórmula universal.
Ver [alineamientos](ALIGNMENT_VALIDATION.md).

## Mediciones y comparación

`Measurement::parse(text,options,reference)` es independiente del filesystem y
solver. Las escalas de unidades son explícitas; SPL se refiere a 20 µPa sólo
cuando se selecciona SplDb. Sin fase el tipo devuelve None; conversión compleja
no representable devuelve error. Las frecuencias deben aumentar estrictamente;
no se eliminan duplicados silenciosamente. ZMA interpola en el plano complejo;
FRD evita interpolación directa de fase envuelta. No extrapola ni convierte una
medición de referencia desconocida en respuesta simulada.

`tools/cross-validation/run.py` ejecuta ambos motores fuera del grafo de
producción. `compare.py` acepta exports CSV normalizados, con celdas vacías para
magnitudes no disponibles y tolerancias explícitas. Sus resultados son
comparación de implementaciones, no una nueva verdad física.

## Compatibilidad y madurez

- Sin dependencias nuevas; Rust 1.75 y WASM conservados.
- Constructores predeterminados: bobina clásica, Rf=0, factores de calibración intactos.
- Literales `Driver`: añadir `coil_model: CoilModel::Inductance`.
- Matches exhaustivos de Design/TopologyPoint: añadir TransmissionLine.
- TerminationKind añade PrescribedResistance; una carga arbitraria no es adaptada.
- Nuevo tipo TL: validación analítica/energética, experimental frente a oráculos externos.
- No hay mediciones ZMA verificadas para validar ajuste experimental de Ke.
