//! Offset driver horn (OD / OD1 / OD2).
//!
//! En una bocina de driver desplazado el diafragma no está en la garganta:
//! está conectado en una toma intermedia (`S2`, `S3` o `S4`). El tramo entre
//! la garganta cerrada `S1` y la toma es un stub; el resto de la bocina
//! continúa hasta la boca `S5`.
//!
//! La red se resuelve con matrices de transmisión:
//!
//! ```text
//! Z_stub  = impedancia del tramo S1→toma, con S1 cerrado
//! Z_path  = impedancia del tramo toma→S5, terminada en la radiación
//! Z_horn  = Z_stub ∥ Z_path
//! Z_rear  = cámara trasera (o radiación directa del lado trasero)
//! U_d     = ps / (Zd + Z_rear + Z_horn)
//! ```
//!
//! La boca se alimenta con `p_toma / Z_path` y se propaga hasta `S5`.
//!
//! El lado trasero del diafragma puede ser una cámara sellada o un radiador
//! directo. En este último caso la presión de campo lejano se calcula a partir
//! de la velocidad del diafragma y de la resistencia de radiación del pistón,
//! con el factor de directividad de potencia que usa el software de referencia para su salida
//! *Direct Radiator*:
//!
//! ```text
//! p_trasera = (rho·omega·|U_d| / Omega) · r(ka) · sqrt(escala)
//! r(ka)     = sqrt(2·R1) / (ka·(1 + R1))
//! ```
//!
//! donde `R1` es la parte real normalizada de la impedancia de radiación del
//! pistón. El factor vale 1 a baja frecuencia y tiende a `sqrt(2)/(2·ka)` a
//! alta frecuencia. La calibración global de OD está en `OffsetHorn::base`.

use num_complex::Complex64;

use crate::air::Air;
use crate::driver::Driver;
use crate::element::{parallel, Chamber, PassiveRadiator, Port};
use crate::error::{
    check_frequency, require_finite, require_non_negative, require_polarity, require_positive,
    SolveError,
};
use crate::omega;
use crate::radiation::piston_radiation_impedance;
use crate::segment::{Segment, TransferMatrix};

/// Punto de respuesta de una bocina de driver desplazado.
#[derive(Clone, Copy, Debug)]
pub struct OffsetPoint {
    /// Frecuencia (Hz).
    pub freq: f64,
    /// SPL de potencia a 1 m (dB re 20 µPa).
    pub spl_db: f64,
    /// Fase reducida de la presión TOTAL (grados); presión física añade +90°.
    pub phase_deg: f64,
    /// Módulo de la impedancia eléctrica (Ω).
    pub ze_mag: f64,
    /// Fase de la impedancia eléctrica (grados).
    pub ze_phase_deg: f64,
    /// Desplazamiento de pico del diafragma (mm).
    pub displacement_mm: f64,
    /// Potencia acústica radiada (W).
    pub power_w: f64,
    /// Rendimiento acústico (%).
    pub efficiency_pct: f64,
    pub port_velocity: Option<f64>,
}

impl OffsetPoint {
    /// Comprueba que el punto sea físicamente interpretable.
    pub fn validate(&self) -> Result<(), SolveError> {
        if !self.freq.is_finite() || self.freq <= 0.0 {
            return Err(SolveError::InvalidResponse("freq"));
        }
        if !(self.spl_db.is_finite() || self.spl_db == f64::NEG_INFINITY) {
            return Err(SolveError::InvalidResponse("spl_db"));
        }
        if !self.phase_deg.is_finite() {
            return Err(SolveError::InvalidResponse("phase_deg"));
        }
        if !(self.ze_mag.is_finite() && self.ze_mag > 0.0) {
            return Err(SolveError::InvalidResponse("ze_mag"));
        }
        if !self.ze_phase_deg.is_finite() {
            return Err(SolveError::InvalidResponse("ze_phase_deg"));
        }
        if !(self.displacement_mm.is_finite() && self.displacement_mm >= 0.0) {
            return Err(SolveError::InvalidResponse("displacement_mm"));
        }
        if !(self.power_w.is_finite() && self.power_w >= 0.0) {
            return Err(SolveError::InvalidResponse("power_w"));
        }
        if !(self.efficiency_pct.is_finite() && self.efficiency_pct >= 0.0) {
            return Err(SolveError::InvalidResponse("efficiency_pct"));
        }
        if let Some(velocity) = self.port_velocity {
            if !(velocity.is_finite() && velocity >= 0.0) {
                return Err(SolveError::InvalidResponse("port_velocity"));
            }
        }
        Ok(())
    }
}

/// Bocina de driver desplazado.
#[derive(Clone, Debug)]
pub struct OffsetHorn {
    /// Propiedades del aire.
    pub air: Air,
    /// Altavoz.
    pub driver: Driver,
    /// Tramos S1→S2, S2→S3, … en orden.
    pub segments: Vec<Segment>,
    /// Nodo de la toma: 1 = S2, 2 = S3, 3 = S4, 0 = garganta.
    pub tap: usize,
    /// Volumen de la cámara trasera (m³). `None` = radiación directa trasera.
    ///
    /// Si es `Some`, debe ser finito y positivo.
    pub rear_chamber_m3: Option<f64>,
    /// Puerto de la cámara trasera (opcional).
    ///
    /// Requiere `rear_chamber_m3: Some`.
    pub rear_port: Option<Port>,
    /// Radiador pasivo de la cámara trasera (opcional).
    ///
    /// Requiere `rear_chamber_m3: Some`.
    pub rear_passive_radiator: Option<PassiveRadiator>,
    /// Factor de calibración de la **potencia** radiada por la parte trasera
    /// directa. No afecta a la carga del diafragma, solo al SPL.
    pub rear_radiation_scale: f64,
    /// Ángulo sólido de radiación (estereorradianes).
    pub solid_angle: f64,
    /// Tensión de entrada (V); el signo invierte la polaridad.
    pub eg: f64,
    /// Resistencia de fuente (Ω).
    pub rg: f64,
    /// Escala de la impedancia de radiación de la boca.
    pub mouth_radiation_scale: f64,
    /// Factor de área para la radiación de la boca.
    pub mouth_radiation_area_factor: f64,
    /// Ajuste global del SPL para la calibración del software de referencia 60.70.
    pub spl_offset_db: f64,
    /// Polaridad del driver (+1 o -1).
    pub polarity: f64,
    /// Incluir la masa añadida del aire en la impedancia mecánica.
    pub include_air_load_mass: bool,
}

/// Producto `left · right` de matrices de transmisión.
fn cascade(left: TransferMatrix, right: TransferMatrix) -> TransferMatrix {
    TransferMatrix {
        a: left.a * right.a + left.b * right.c,
        b: left.a * right.b + left.b * right.d,
        c: left.c * right.a + left.d * right.c,
        d: left.c * right.b + left.d * right.d,
    }
}

impl OffsetHorn {
    /// OD de 4 tramos con la toma en S2.
    pub fn new_4seg_od(driver: Driver, segments: Vec<Segment>) -> Self {
        let mut od = Self::base(driver, segments);
        od.tap = 1;
        od
    }

    /// OD1 de 4 tramos con la toma en S3.
    pub fn new_4seg_od1(driver: Driver, segments: Vec<Segment>) -> Self {
        let mut od = Self::base(driver, segments);
        od.tap = 2;
        od
    }

    /// OD2 de 4 tramos con la toma en S4.
    pub fn new_4seg_od2(driver: Driver, segments: Vec<Segment>) -> Self {
        let mut od = Self::base(driver, segments);
        od.tap = 3;
        od
    }

    /// Calibración común de OD/OD1/OD2 contra el software de referencia 60.70.
    ///
    /// Los cuatro valores se ajustaron con los tres exports de producción
    /// (`od_a`, `od1_b`, `od2_c`) y se mantienen iguales para toda la
    /// topología. La banda validada es 10 Hz–1 kHz. Son factores empíricos
    /// de calibración, no constantes físicas: para otra geometría conviene
    /// validar contra su propio export.
    fn base(driver: Driver, segments: Vec<Segment>) -> Self {
        Self {
            air: Air::default(),
            driver,
            segments,
            tap: 1,
            rear_chamber_m3: None,
            rear_port: None,
            rear_passive_radiator: None,
            rear_radiation_scale: 4.50,
            solid_angle: 0.5 * std::f64::consts::PI,
            eg: 2.83,
            rg: 0.0,
            mouth_radiation_scale: 4.50,
            mouth_radiation_area_factor: 4.75,
            spl_offset_db: -5.75,
            polarity: 1.0,
            include_air_load_mass: true,
        }
    }

    /// Comprueba la configuración de la bocina de driver desplazado.
    ///
    /// Exige tramos válidos, toma dentro de rango, cámara trasera positiva si
    /// existe, y puerto/radiador pasivo solo con cámara trasera.
    pub fn validate(&self) -> Result<(), SolveError> {
        self.air.validate()?;
        self.driver.validate()?;
        if self.segments.is_empty() {
            return Err(SolveError::InvalidTopology("bocina sin segmentos"));
        }
        for segment in &self.segments {
            segment.validate()?;
        }
        if self.tap > self.segments.len() {
            return Err(SolveError::InvalidTopology("toma fuera de los tramos"));
        }
        if let Some(volume) = self.rear_chamber_m3 {
            require_positive(volume, "rear_chamber_m3")?;
        }
        if let Some(port) = &self.rear_port {
            port.validate()?;
            if self.rear_chamber_m3.is_none() {
                return Err(SolveError::InvalidTopology(
                    "rear_port requiere rear_chamber_m3",
                ));
            }
        }
        if let Some(radiator) = &self.rear_passive_radiator {
            radiator.validate()?;
            if self.rear_chamber_m3.is_none() {
                return Err(SolveError::InvalidTopology(
                    "rear_passive_radiator requiere rear_chamber_m3",
                ));
            }
        }
        require_non_negative(self.rear_radiation_scale, "rear_radiation_scale")?;
        require_positive(self.solid_angle, "solid_angle")?;
        require_finite(self.eg, "eg")?;
        require_non_negative(self.rg, "rg")?;
        require_positive(self.mouth_radiation_scale, "mouth_radiation_scale")?;
        require_positive(
            self.mouth_radiation_area_factor,
            "mouth_radiation_area_factor",
        )?;
        require_finite(self.spl_offset_db, "spl_offset_db")?;
        require_polarity(self.polarity, "polarity")?;
        Ok(())
    }

    /// Impedancia de radiación de la boca.
    fn mouth_impedance(&self, f: f64) -> Complex64 {
        let area =
            self.segments.last().map(|s| s.s2()).unwrap_or(0.0) * self.mouth_radiation_area_factor;
        piston_radiation_impedance(&self.air, area, f, self.solid_angle)
            * self.mouth_radiation_scale
    }

    /// Impedancia del stub S1→toma, con S1 cerrado.
    ///
    /// Se obtiene cascando las matrices de los tramos S1→…→toma y cerrando
    /// la entrada (`U1 = 0`): `Z = −A_total/C_total`.
    pub fn stub_impedance(&self, f: f64) -> Complex64 {
        if self.tap == 0 || self.tap > self.segments.len() {
            return Complex64::new(f64::INFINITY, 0.0);
        }
        let mut total = TransferMatrix::identity();
        for seg in &self.segments[..self.tap] {
            let m = seg.matrix(&self.air, f);
            total = cascade(m, total);
        }
        if total.c.norm() > 1.0e-30 {
            -total.a / total.c
        } else {
            Complex64::new(f64::INFINITY, 0.0)
        }
    }

    /// Impedancia del camino toma→boca, terminada en la radiación.
    pub fn path_impedance(&self, f: f64) -> Complex64 {
        if self.tap >= self.segments.len() {
            return self.mouth_impedance(f);
        }
        let mut z = self.mouth_impedance(f);
        for seg in self.segments[self.tap..].iter().rev() {
            z = seg.matrix(&self.air, f).transform_impedance(z);
        }
        z
    }

    /// Impedancia de la bocina en la toma: stub ∥ camino a la boca.
    pub fn horn_impedance(&self, f: f64) -> Complex64 {
        let z_path = self.path_impedance(f);
        if self.tap == 0 {
            return z_path;
        }
        let z_stub = self.stub_impedance(f);
        if z_stub.norm().is_infinite() {
            return z_path;
        }
        parallel(z_stub, z_path)
    }

    /// Impedancia de carga del lado trasero del diafragma.
    pub fn rear_impedance(&self, f: f64) -> Complex64 {
        let mut z = match self.rear_chamber_m3 {
            Some(volume) => Chamber::new(volume).impedance(&self.air, f),
            None => piston_radiation_impedance(&self.air, self.driver.sd, f, self.solid_angle),
        };
        if let Some(port) = &self.rear_port {
            z = parallel(z, port.impedance(&self.air, f));
        }
        if let Some(pr) = &self.rear_passive_radiator {
            z = parallel(z, pr.impedance(f));
        }
        z
    }

    /// Impedancia mecánica del driver (con o sin masa añadida de aire).
    fn driver_zm(&self, f: f64) -> Complex64 {
        let mut zm = self.driver.mechanical_impedance(f);
        if self.include_air_load_mass {
            zm += Complex64::new(0.0, omega(f) * self.driver.air_load_mass(&self.air));
        }
        zm
    }

    /// Respuesta a una frecuencia. Devuelve un punto neutro si la entrada o el
    /// sistema no son resolubles; use [`OffsetHorn::try_response_at`] para
    /// distinguir el error.
    pub fn response_at(&self, f: f64) -> OffsetPoint {
        self.try_response_at(f)
            .unwrap_or_else(|_| Self::zero_point(f))
    }

    /// Respuesta a una frecuencia, con error explícito si la entrada, la
    /// configuración o el resultado no son válidos.
    pub fn try_response_at(&self, f: f64) -> Result<OffsetPoint, SolveError> {
        check_frequency(f)?;
        self.validate()?;
        let point = self.solve_point(f)?;
        point.validate()?;
        Ok(point)
    }

    /// Punto neutro para una entrada o un sistema no resoluble.
    fn zero_point(f: f64) -> OffsetPoint {
        OffsetPoint {
            freq: f,
            spl_db: f64::NEG_INFINITY,
            phase_deg: 0.0,
            ze_mag: 0.0,
            ze_phase_deg: 0.0,
            displacement_mm: 0.0,
            power_w: 0.0,
            efficiency_pct: 0.0,
            port_velocity: None,
        }
    }

    /// Resuelve la red a la frecuencia `f` sin validar la entrada.
    fn solve_point(&self, f: f64) -> Result<OffsetPoint, SolveError> {
        let z_horn = self.horn_impedance(f);
        let z_rear = self.rear_impedance(f);
        let zm = self.driver_zm(f);
        let ze_coil = self.driver.coil_impedance(f) + Complex64::new(self.rg, 0.0);
        let zd = (zm + Complex64::new(self.driver.bl * self.driver.bl, 0.0) / ze_coil)
            / (self.driver.sd * self.driver.sd);
        let z_total = zd + z_rear + z_horn;
        if z_total.norm() < 1.0e-30 {
            return Err(SolveError::SingularSystem);
        }
        let ps = Complex64::new(self.driver.bl * self.eg / self.driver.sd, 0.0) / ze_coil
            * self.polarity;
        let u_d = ps / z_total;

        // Presión en la toma y velocidad que entra en el camino a la boca.
        let p_tap = z_horn * u_d;
        let z_path = self.path_impedance(f);
        let u_path = if z_path.norm() > 1.0e-30 {
            p_tap / z_path
        } else {
            Complex64::new(0.0, 0.0)
        };

        // Propagar el estado (p, U) desde la toma hasta la boca.
        let mut p = p_tap;
        let mut u = u_path;
        for seg in &self.segments[self.tap..] {
            let (p2, u2) = seg.matrix(&self.air, f).apply(p, u);
            p = p2;
            u = u2;
        }
        let u_mouth = u;
        let z_mouth = self.mouth_impedance(f);

        // Presión de la boca de la bocina (magnitud de potencia) y presión
        // del radiador trasero directo. La parte trasera solo radica cuando
        // no hay cámara trasera ni puerto/radiador pasivo.
        let p_ac_horn =
            u_mouth.norm_sqr() * z_mouth.re * (std::f64::consts::TAU / self.solid_angle);
        let p_horn_mag = if p_ac_horn > 0.0 {
            (p_ac_horn * self.air.rho * self.air.c / self.solid_angle).sqrt()
        } else {
            0.0
        };
        let p_horn = Complex64::from_polar(p_horn_mag, u_mouth.arg());

        let rear_direct = self.tap > 0
            && self.rear_chamber_m3.is_none()
            && self.rear_port.is_none()
            && self.rear_passive_radiator.is_none();
        let z_rear_rad = piston_radiation_impedance(&self.air, self.driver.sd, f, self.solid_angle);
        // Presión en campo lejano del radiador trasero. La resistencia de
        // radiación del pistón da el factor de directividad de potencia
        // `r = sqrt(2·R1) / (ka·(1 + R1))`, que vale 1 a baja frecuencia y
        // tiende a `sqrt(2)/(2·ka)` cuando `ka` crece. Así el radiador trasero
        // reproduce la "presión de directividad constante" del software de referencia.
        let r1 = if self.driver.sd > 0.0 {
            z_rear_rad.re * self.driver.sd / (self.air.rho * self.air.c)
        } else {
            0.0
        };
        let radius = (self.driver.sd / std::f64::consts::PI).sqrt();
        let ka = omega(f) / self.air.c * radius;
        let directivity = if ka > 1.0e-9 {
            (2.0 * r1).max(0.0).sqrt() / (ka * (1.0 + r1))
        } else {
            1.0
        };
        let p_rear_mag = if rear_direct {
            self.air.rho * omega(f) * u_d.norm() / self.solid_angle
                * directivity
                * self.rear_radiation_scale.sqrt()
        } else {
            0.0
        };
        let p_ac_rear = if p_rear_mag > 0.0 {
            p_rear_mag * p_rear_mag * self.solid_angle / (self.air.rho * self.air.c)
        } else {
            0.0
        };
        let p_rear = if rear_direct {
            Complex64::from_polar(p_rear_mag, (-u_d).arg())
        } else {
            Complex64::new(0.0, 0.0)
        };
        let p_total = p_horn + p_rear;
        // Salidas traseras: el caudal saliente es opuesto al del diafragma.
        // Aproximación compacta a 1 m: igual distancia y fase de propagación.
        // No se asigna radiación a las pérdidas de cámara/PR.
        let mut p_total = p_total;
        let mut p_ac = p_ac_horn + p_ac_rear;
        if self.rear_chamber_m3.is_some() {
            let p_chamber = -u_d * z_rear;
            let outlets = self
                .rear_port
                .map(|port| (port.area, port.impedance(&self.air, f)))
                .into_iter()
                .chain(
                    self.rear_passive_radiator
                        .map(|pr| (pr.sd, pr.impedance(f))),
                );
            for (area, z) in outlets {
                let pressure = crate::system::radiator_pressure(
                    &self.air,
                    area,
                    p_chamber / z,
                    f,
                    self.solid_angle,
                );
                p_total += pressure;
                p_ac += pressure.norm_sqr() * self.solid_angle / (self.air.rho * self.air.c);
            }
        }
        let p_rms = p_total.norm();
        let spl = if p_rms > 0.0 {
            20.0 * (p_rms / self.air.p_ref).log10() + self.spl_offset_db
        } else {
            f64::NEG_INFINITY
        };

        let ze = ze_coil
            + Complex64::new(self.driver.bl * self.driver.bl, 0.0)
                / (zm + self.driver.sd * self.driver.sd * (z_horn + z_rear));
        let current = Complex64::new(self.eg, 0.0) / ze;
        let p_in = self.eg * current.re;
        let disp_mm =
            (u_d.norm() / (omega(f) * self.driver.sd)) * 1000.0 * std::f64::consts::SQRT_2;

        let port_velocity = match (&self.rear_port, self.rear_chamber_m3) {
            (Some(port), Some(_)) => {
                let p_rear = z_rear * u_d;
                let u_port = p_rear / port.impedance(&self.air, f);
                Some((u_port / port.area).norm())
            }
            _ => None,
        };

        Ok(OffsetPoint {
            freq: f,
            spl_db: spl,
            phase_deg: p_total.arg().to_degrees(),
            ze_mag: ze.norm(),
            ze_phase_deg: ze.arg().to_degrees(),
            displacement_mm: disp_mm,
            power_w: p_ac,
            efficiency_pct: if p_in > 0.0 { 100.0 * p_ac / p_in } else { 0.0 },
            port_velocity,
        })
    }

    /// Barrido de respuesta. Se eliminan las frecuencias no finitas o no
    /// positivas.
    pub fn response(&self, freqs: &[f64]) -> Vec<OffsetPoint> {
        freqs
            .iter()
            .copied()
            .filter(|f| f.is_finite() && *f > 0.0)
            .map(|f| self.response_at(f))
            .collect()
    }

    /// Número de nodos de la red (S1..S5).
    pub fn node_count(&self) -> usize {
        self.segments.len() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::EndCorrection;
    use crate::segment::Flare;

    fn driver() -> Driver {
        Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
    }

    fn od() -> OffsetHorn {
        let mut od = OffsetHorn::new_4seg_od(
            driver(),
            vec![
                Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
                Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
                Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
                Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
            ],
        );
        od.rear_chamber_m3 = Some(10.0e-3);
        od
    }

    #[test]
    fn closed_stub_of_short_tube_is_a_compliance() {
        let air = Air::default();
        let seg = Segment::new(Flare::Cylindrical, 0.40, 400e-4, 400e-4);
        let od = OffsetHorn::new_4seg_od(driver(), vec![seg]);
        let f = 1.0;
        let z = od.stub_impedance(f);
        let c = 400e-4 * 0.40 / (air.rho * air.c * air.c);
        let expected = Complex64::new(0.0, -1.0) / (omega(f) * c);
        assert!(
            (z - expected).norm() / expected.norm() < 1.0e-4,
            "{z:?} vs {expected:?}"
        );
    }

    #[test]
    fn offset_horn_is_finite() {
        let od = od();
        for f in [10.0, 20.0, 50.0, 100.0, 500.0, 2000.0] {
            let p = od.response_at(f);
            assert!(p.spl_db.is_finite(), "SPL no finito a {f} Hz");
            assert!(p.ze_mag.is_finite() && p.ze_mag > 0.0);
            assert!(p.displacement_mm.is_finite() && p.displacement_mm >= 0.0);
            assert!(p.power_w >= 0.0);
        }
    }

    #[test]
    fn larger_rear_chamber_increases_low_frequency_excursion() {
        let mut small = od();
        small.rear_chamber_m3 = Some(5.0e-3);
        let mut large = od();
        large.rear_chamber_m3 = Some(40.0e-3);
        let xs = small.response_at(20.0).displacement_mm;
        let xl = large.response_at(20.0).displacement_mm;
        assert!(xl > xs, "Xd(40 L)={xl} <= Xd(5 L)={xs}");
    }

    #[test]
    fn multisegment_stub_matches_transfer_matrix() {
        let air = Air::default();
        let segments = vec![
            Segment::new(Flare::Conical, 0.25, 300e-4, 500e-4),
            Segment::new(Flare::Conical, 0.35, 500e-4, 500e-4),
        ];
        let od = OffsetHorn::new_4seg_od1(driver(), segments.clone());
        for f in [10.0, 20.0, 50.0, 100.0, 200.0, 500.0] {
            let m1 = segments[0].matrix(&air, f);
            let m2 = segments[1].matrix(&air, f);
            let total = cascade(m2, m1);
            let expected = -total.a / total.c;
            let got = od.stub_impedance(f);
            assert!(
                (got - expected).norm() / expected.norm() < 1.0e-12,
                "f={f}: {got:?} vs {expected:?}"
            );
        }
    }

    #[test]
    fn tap_at_throat_matches_normal_horn() {
        use crate::element::Chamber;
        use crate::segment::{Flare, Horn};
        use crate::system::System;

        let segments = vec![
            Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
        ];
        let mut od = OffsetHorn::new_4seg_od(driver(), segments.clone());
        od.tap = 0;
        od.rear_chamber_m3 = Some(10.0e-3);
        od.include_air_load_mass = false;
        od.mouth_radiation_scale = 1.0;
        od.mouth_radiation_area_factor = 1.0;
        od.spl_offset_db = 0.0;
        od.solid_angle = std::f64::consts::TAU;

        let mut sys = System::new(Air::default(), driver());
        sys.horn = Some(Horn::new(segments));
        sys.rear_chamber = Some(Chamber::new(10.0e-3));
        sys.solid_angle = std::f64::consts::TAU;
        sys.eg = od.eg;

        for f in [20.0, 50.0, 100.0, 500.0] {
            let a = od.response_at(f);
            let b = sys.response_at(f);
            assert!(
                (a.spl_db - b.spl_db).abs() < 1.0e-9,
                "SPL {f} Hz: {} vs {}",
                a.spl_db,
                b.spl_db
            );
            assert!(
                (a.ze_mag - b.ze_mag).abs() / b.ze_mag < 1.0e-9,
                "Ze {f} Hz: {} vs {}",
                a.ze_mag,
                b.ze_mag
            );
        }
    }

    #[test]
    fn moving_the_tap_changes_the_response() {
        let a = OffsetHorn::new_4seg_od(driver(), od().segments.clone());
        let b = OffsetHorn::new_4seg_od1(driver(), od().segments.clone());
        assert!((a.response_at(100.0).spl_db - b.response_at(100.0).spl_db).abs() > 0.1);
    }

    #[test]
    fn try_response_at_rejects_invalid_frequency() {
        let horn = od();
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                horn.try_response_at(f),
                Err(SolveError::InvalidFrequency(_))
            ));
        }
        assert!(horn.try_response_at(100.0).is_ok());
    }

    #[test]
    fn try_response_at_rejects_invalid_tap_and_chamber() {
        let mut horn = od();
        horn.tap = 99;
        assert!(matches!(
            horn.try_response_at(100.0),
            Err(SolveError::InvalidTopology(_))
        ));

        let mut horn = od();
        horn.rear_chamber_m3 = Some(0.0);
        assert!(matches!(
            horn.try_response_at(100.0),
            Err(SolveError::InvalidParameter("rear_chamber_m3"))
        ));

        let mut horn = od();
        horn.rear_chamber_m3 = None;
        horn.rear_port = Some(Port::new(50.0e-4, 0.10, EndCorrection::UnflangedBoth));
        assert!(matches!(
            horn.try_response_at(100.0),
            Err(SolveError::InvalidTopology(_))
        ));
    }

    #[test]
    fn point_validation_rejects_invalid_fields() {
        let valid = od().response_at(100.0);
        assert!(valid.validate().is_ok());

        let mut p = valid;
        p.ze_mag = f64::NAN;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.power_w = -1.0;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.port_velocity = Some(f64::INFINITY);
        assert!(p.validate().is_err());
    }

    #[test]
    fn response_filters_non_finite_frequencies() {
        let pts = od().response(&[f64::NAN, f64::INFINITY, 0.0, -1.0, 100.0]);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].freq - 100.0).abs() < 1.0e-12);
    }
}
