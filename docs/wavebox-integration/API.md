# API y formatos

## Red y solvers

`network::AcousticNetwork` contiene nodos indexados desde cero, conductos y
fronteras. Para un conducto distribuido use `Conduit::Distributed(Segment)`,
también para una cámara de sección constante con longitud explícita. Para un
recinto concentrado conecte `Boundary::Chamber` a un nodo. Las aberturas
radiantes y resistencias se modelan como fronteras. Un extremo de conducto
sin frontera ni conexión adicional tiene caudal cero. Los constructores
`NetworkLayout::paraflex_ph1`, `double_bass_reflex`,
`bandpass_eighth_series` y `bandpass_b` devuelven también los nodos del
diafragma. Los parámetros se introducen en SI.

`NetworkLayout::{from_json,to_json}` permiten persistir la geometría básica
en JSON v1. La excitación, aire y motor se suministran por separado. Los
conductos pueden ser inerciales, cilíndricos, cónicos, exponenciales,
de área lineal o Hypex. Se serializan pérdidas Darcy y pared cuando existen.
Para un contorno `Custom`, conserve su `HornProfileSpec` o datos
geométricos por separado. La red en memoria admite ese perfil, termoviscosidad
y revestimientos de cámara. El formato v1 devuelve un error explícito
cuando no puede persistirlos. Consulte los ejemplos [PH1](ph1-network.json)
y [BP4](bp4-network.json).

## Maximum SPL y EQ

`UnitDriveResponse::from_design` usa la respuesta nativa a 1 V RMS en el
generador. `from_network` normaliza un resultado complejo de la red por
su tensión. `maximum_spl` obtiene la mínima tensión que respeta la excursión
de pico, la potencia (activa en bornes o térmica de bobina) y la tensión RMS
en circuito abierto del amplificador. Los voltajes por límite y la restricción
dominante se exponen junto al punto de salida y la calidad radiativa. Con una
restricción no caracterizada se informa su ausencia. Un límite cero extingue
la salida si existe sensibilidad a ese límite. La ausencia de un límite
finito y vinculante se informa como error. La potencia en bornes usa la
parte real de `V·conj(I)`; la potencia de bobina usa la resistencia real
bloqueada a la frecuencia de interés. `Rg` sólo pertenece al generador.

```rust
use speakerbox_acoustics::{Design, SolveError};
use speakerbox_acoustics::maximum_spl::{MaximumSplLimits, PowerBasis, UnitDriveResponse};

fn max_db(design: &Design, frequency_hz: f64) -> Result<f64, SolveError> {
    let unit = UnitDriveResponse::from_design(design, frequency_hz)?;
    let point = unit.maximum_spl(MaximumSplLimits {
        excursion_peak_m: Some(0.006),
        power_limit: Some((PowerBasis::CoilHeat, 180.0)),
        amplifier_open_circuit_v_rms: Some(28.0),
    })?;
    Ok(point.output.spl_db)
}
```

`speakerbox_circuits::equalizer::equalizer_response` devuelve el producto
complejo de etapas analógicas idealizadas, de pico o shelf. Use
`H(f)·V_generator(f)` como `NetworkDriver::voltage_rms` o suminístrelo a
`UnitDriveResponse::at_voltage` para actualizar SPL, fase, excursión y
potencia. Un filtro pasivo cuya entrada y salida soporten cargas acopladas
se analiza mediante MNA. El escalado describe la salida lineal estacionaria.
La compresión térmica, turbulencia, distorsión y limitación del amplificador
por corriente requieren datos o modelos separados.

## Interoperabilidad Hornresp

`record::HornrespTextRecord::parse` lee campos por sección y permite recuperar
el texto original byte por byte, incluso para formatos fuera de la conversión.
Los parámetros opcionales de semiinductancia y FDD se leen sólo cuando sus
banderas los activan. Un `Le` en el bloque tradicional conserva su
significado separado del `Le` en el bloque avanzado.

Con una plantilla sellada o ventilada de un solo driver tradicional,
`simple_system` importa parámetros soportados.
`export_simple_system` actualiza esos mismos campos de una plantilla
compatible. Usa cm², cm, litros, gramos y mH donde los requiere el registro.
Respeta el `Lrc`, límites de salida y campos de plantilla no editados.
El comportamiento de cámara y puerto es la aproximación del `System`
tradicional. El formato de red arbitraria se intercambia en JSON y no
tiene una conversión general a Hornresp. El round-trip del subconjunto
textual se validó con dos plantillas existentes. Queda pendiente confirmar
registros generados con una instalación externa de Hornresp.
