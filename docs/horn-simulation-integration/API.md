# API y compatibilidad

## Geometría

```rust
use speakerbox_acoustics::{HornProfileKind, HornProfileSpec, SolveError};

fn example() -> Result<(), SolveError> {
    let spec = HornProfileSpec {
        kind: HornProfileKind::OblateSpheroidal,
        length_m: 0.30,
        throat_radius_m: 0.025,
        mouth_radius_m: 0.120,
    };
    let json = spec.to_json()?;
    let profile = HornProfileSpec::from_json(&json)?.resolve()?;
    let sections = profile.sample(201)?;
    let volume_m3 = profile.volume_m3();
    let horn = profile.into_horn();
    assert_eq!(sections.len(), 201);
    assert!(volume_m3 > 0.0);
    assert_eq!(horn.segments.len(), 1);
    Ok(())
}
```

JSON v1 requiere `version`, `kind`, `length_m`, `throat_radius_m` y
`mouth_radius_m`. Los nombres admitidos son `cylindrical`, `conical`,
`exponential`, `hypex`, `tractrix`, `scaled_tractrix` y
`oblate_spheroidal` y `exponential_conical`. Hypex requiere además `t`.
Exponencial-cónica requiere `join_fraction` en el intervalo abierto
`(0,1)`; el área y su primera derivada coinciden en la unión. La deserialización
valida datos y dominio. El perfil resuelto incluye extremos, muestras
`(x,r,S)` y volumen. El CAD puede muestrear posiciones propias con
`section_at`, por ejemplo para concentrar puntos cerca de una boca
tractrix completa, cuya pendiente es vertical.

Se eligió `HornProfileSpec` externo a `Flare` para conservar compatibilidad
con los consumidores que hacen `match` exhaustivo. Los perfiles básicos
usan sus mismos segmentos. OS y las tractrices generan un `Flare::Custom`
a partir de una especificación persistente. La exponencial-cónica compone
dos segmentos nativos. El volumen es geométrico, con integración analítica para
los perfiles nuevos y para cilindro, cono y exponencial. Hypex delega su
integración a `Segment::volume`. `Horn::volume` usa la cuadratura histórica,
por lo que puede diferir ligeramente del volumen analítico de una tractrix.

## Cámara frontal y pérdidas

`System::with_front_adapter(FrontAdapter { volume_m3, area_m2 })` inserta una
sección cilíndrica ideal antes de la garganta. Con volumen cero conserva la
conexión previa. Una cámara, fuga o resistencia concentrada ya presente se
considera un conflicto para un volumen nuevo positivo. La cara del
adaptador es el plano de entrada de la bocina compuesta.
`System::try_horn_operating_point_at` expone fasores RMS de entrada, velocidad
del diafragma y caudal de boca. Para recuperar caudal y presión en la garganta
original, aplique la matriz del primer segmento a los fasores de entrada.
Este modelo supone onda plana y continuidad de presión y caudal en los saltos
de área. Una cámara con fase de orden alto, forma multicanal o área transversal
grande frente a la longitud de onda requiere otro modelo. Como criterio de
uso conservador, calcule `ka` con el radio mayor del acoplamiento y úselo
preferentemente por debajo de 1.0. La procedencia de área y volumen corresponde
al diseño o medición que suministra esos parámetros.

`Segment::with_thermoviscous_losses(ThermoviscousAir::default())` activa la
opción de capa delgada en un cilindro. `thermoviscous_properties` devuelve los
parámetros usados y la subdivisión acústica de `TransmissionLine` los conserva.
El dominio se comprueba por frecuencia. `System::try_energy_at` registra
disipación en `acoustic_losses.wall_w`, igual que con otras pérdidas
distribuidas. La porción termoviscosa puede calcularse por tramo con
`Segment::dissipated_power_w`. El aire termodinámico debe corresponder al
`Air` elegido. [Modelo y límites](DECISIONES.md#pérdidas-de-capa-límite).

## Distintas fidelidades

`optimization::screen_and_refine` recibe candidatos tipados, un filtro duro,
una función de puntuación inicial y otra de refinado. Por ejemplo, construya
un `HornProfileSpec`, valide longitud y volumen de fabricación, puntúe con
`Design::try_sweep_log` o `acoustic_cost`, y refíne los elegidos con una
función que use `minimize` o invoque un backend externo. Las preferencias de
SPL, coste o peso pertenecen a las funciones de puntuación. Agrupe por
familia o por intervalo geométrico para evaluar primero un candidato por grupo.
Las puntuaciones inicial y refinada quedan disponibles para comparar orden y
diferencias. El límite de cada backend se expresa en llamadas, incluso si
fallan. La cancelación se comprueba entre llamadas. Los backends de larga
duración deben propagar una señal propia a su trabajo interno.

Con un presupuesto inicial que cubra todos los candidatos, el filtro solo
descarta los que incumplen restricciones explícitas. Un presupuesto inicial
parcial recorre el conjunto en orden de entrada y puede omitir al mejor.
El mejor candidato del primer modelo entra primero a refinado si el segundo
presupuesto es positivo. No se garantiza retener al mejor del modelo de
mayor fidelidad cuando ambos modelos ordenan distinto. El test conserva el mejor candidato de una búsqueda exhaustiva con dos
cuadrículas 1D, de 2 y 8 frecuencias, tres familias y un límite de fabricación. La política de exploración de alto coste sigue siendo decisión
del consumidor.
