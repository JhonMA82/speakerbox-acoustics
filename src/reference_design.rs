//! Propuestas iniciales y TL experimental, sin otro solucionador TMM.
use crate::{error::SolveError, Air, Driver, Flare, OffsetHorn, Segment};
#[derive(Clone, Copy, Debug)]
pub struct SealedAlignment {
    pub volume_m3: f64,
    pub resonance_hz: f64,
    pub f3_hz: f64,
    pub qtc: f64,
}
/// Filtro pasaaltos ideal de segundo orden. T/S en SI; no incluye Le ni pérdidas de caja.
/// V=Vas/((Qtc/Qts)²-1), Fc=Fs Qtc/Qts.
/// F3 se obtiene de x⁴+(2-Qtc⁻²)x²-1=0, sin constantes empíricas.
pub fn sealed_alignment(
    fs: f64,
    qts: f64,
    vas: f64,
    qtc: f64,
) -> Result<SealedAlignment, SolveError> {
    for x in [fs, qts, vas, qtc] {
        if !x.is_finite() || x <= 0.0 {
            return Err(SolveError::InvalidParameter("alignment input"));
        }
    }
    if qtc <= qts {
        return Err(SolveError::InvalidParameter("Qtc must exceed Qts"));
    }
    let alpha = (qtc / qts).powi(2) - 1.;
    let fc = fs * qtc / qts;
    let b = 2. - 1. / qtc.powi(2);
    let y = if b >= 0. {
        2. / (b.hypot(2.) + b)
    } else {
        (b.hypot(2.) - b) / 2.
    };
    let a = SealedAlignment {
        volume_m3: vas / alpha,
        resonance_hz: fc,
        f3_hz: fc * y.sqrt(),
        qtc,
    };
    if [a.volume_m3, a.resonance_hz, a.f3_hz]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.)
    {
        return Err(SolveError::InvalidParameter("alignment overflow"));
    }
    Ok(a)
}
/// TL uniforme experimental con extremo inicial cerrado y boca de pistón abierta.
/// Driver en x/L, 0<=x/L<1. Rf en Pa·s/m²; cero conserva el tubo sin pérdidas.
/// El lado exterior del driver radia directamente. Sin pliegues ni cierre de boca.
/// Se reutiliza OffsetHorn; convertir con Design::from para barridos y energía.
pub fn uniform_transmission_line(
    driver: Driver,
    length: f64,
    area: f64,
    position: f64,
    rf: f64,
) -> Result<OffsetHorn, SolveError> {
    if !length.is_finite()
        || length <= 0.
        || !position.is_finite()
        || !(0.0..1.0).contains(&position)
    {
        return Err(SolveError::InvalidParameter("TL geometry"));
    }
    let seg = |l| Segment::new(Flare::Cylindrical, l, area, area).with_flow_resistivity(rf);
    let segments = if position == 0. {
        vec![seg(length)]
    } else {
        vec![seg(length * position), seg(length * (1. - position))]
    };
    let mut line = OffsetHorn::new_4seg_od(driver, segments);
    line.tap = usize::from(position > 0.);
    line.air = Air::default();
    line.solid_angle = std::f64::consts::TAU;
    line.mouth_radiation_scale = 1.;
    line.mouth_radiation_area_factor = 1.;
    line.rear_radiation_scale = 1.;
    line.spl_offset_db = 0.;
    line.include_air_load_mass = false;
    line.validate()?;
    Ok(line)
}
