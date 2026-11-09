//! Tapped horn (TH/TH1).
//!
//! En un tapped horn el diafragma no está en la garganta: está conectado entre
//! dos puntos del recorrido de la bocina (las "tomas"). Para un TH de 4 tramos
//! con la opción TH1 las tomas son S2 y S3; para un TH de 4 tramos normal son
//! S2 y S4. El extremo S1 está cerrado y la boca radiante es el último nodo.
//!
//! El solver plantea análisis nodal sobre las presiones de los nodos. Cada
//! tramo aporta su matriz de transmisión convertida a matriz de admitancias;
//! el driver es una rama de Norton entre las dos tomas y la boca se termina
//! con la impedancia de radiación.
//!
//! Validación: TH (S2/S4) con dos diseños y TH1 (S2/S3) con uno, contra
//! exports propios del software de referencia 60.70 (`tests/th_production.rs`).

use num_complex::Complex64;

use crate::air::Air;
use crate::driver::Driver;
use crate::error::{
    check_frequency, require_finite, require_non_negative, require_polarity, require_positive,
    SolveError,
};
use crate::omega;
use crate::radiation::piston_radiation_impedance;
use crate::segment::Segment;

/// Punto de respuesta de un tapped horn.
#[derive(Clone, Copy, Debug)]
pub struct TappedPoint {
    /// Frecuencia (Hz).
    pub freq: f64,
    /// SPL de potencia a 1 m (dB re 20 µPa).
    pub spl_db: f64,
    /// Fase de la velocidad volumétrica radiada (grados).
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
}

/// Sistema tapped horn.
#[derive(Clone, Debug)]
pub struct TappedHorn {
    /// Propiedades del aire.
    pub air: Air,
    /// Altavoz.
    pub driver: Driver,
    /// Tramos S1→S2, S2→S3, S3→S4, S4→S5 (en ese orden).
    pub segments: Vec<Segment>,
    /// Nodo de la primera toma (1 = S2).
    pub tap_a: usize,
    /// Nodo de la segunda toma (2 = S3, 3 = S4).
    pub tap_b: usize,
    /// Ángulo sólido de radiación.
    pub solid_angle: f64,
    /// Tensión de entrada (V); el signo invierte la polaridad.
    pub eg: f64,
    /// Resistencia de fuente (Ω).
    pub rg: f64,
    /// Escala de la impedancia de radiación de la boca.
    pub mouth_radiation_scale: f64,
    /// Área efectiva de radiación como múltiplo del área geométrica.
    pub mouth_radiation_area_factor: f64,
    /// Corrección global de SPL (dB).
    pub spl_offset_db: f64,
    /// Polaridad del driver (+1 o -1).
    pub polarity: f64,
}

impl TappedPoint {
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
        Ok(())
    }
}

impl TappedHorn {
    /// Crea un TH de 4 tramos (tomas en S2 y S4) con los valores de
    /// calibración del software de referencia 60.70. Es el caso de los TH1/TH2 exportados.
    pub fn new_4seg_th(driver: Driver, segments: Vec<Segment>) -> Self {
        let mut th = Self::base(driver, segments);
        th.tap_a = 1;
        th.tap_b = 3;
        th
    }

    /// Crea un TH1 de 4 tramos (tomas en S2 y S3).
    pub fn new_4seg_th1(driver: Driver, segments: Vec<Segment>) -> Self {
        let mut th = Self::base(driver, segments);
        th.tap_a = 1;
        th.tap_b = 2;
        th
    }

    fn base(driver: Driver, segments: Vec<Segment>) -> Self {
        Self {
            air: Air::default(),
            driver,
            segments,
            tap_a: 1,
            tap_b: 2,
            solid_angle: 0.5 * std::f64::consts::PI,
            eg: 2.83,
            rg: 0.0,
            mouth_radiation_scale: 3.20,
            mouth_radiation_area_factor: 3.00,
            spl_offset_db: -4.91,
            polarity: 1.0,
        }
    }

    fn node_count(&self) -> usize {
        self.segments.len() + 1
    }

    /// Comprueba la configuración del tapped horn antes de calcular.
    ///
    /// Exige al menos dos tramos válidos, tomas distintas dentro de rango,
    /// aire y driver físicos, y factores de calibración finitos y positivos.
    pub fn validate(&self) -> Result<(), SolveError> {
        self.air.validate()?;
        self.driver.validate()?;
        if self.segments.len() < 2 {
            return Err(SolveError::InvalidTopology(
                "un tapped horn necesita al menos dos tramos",
            ));
        }
        for segment in &self.segments {
            segment.validate()?;
        }
        let n = self.node_count();
        if self.tap_a == 0
            || self.tap_b == 0
            || self.tap_a >= n
            || self.tap_b >= n
            || self.tap_a == self.tap_b
        {
            return Err(SolveError::InvalidTopology(
                "tomas fuera de rango o iguales",
            ));
        }
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

    /// Impedancia mecánica del driver incluyendo la masa añadida del aire.
    fn driver_zm(&self, f: f64) -> Complex64 {
        self.driver.mechanical_impedance(f)
            + Complex64::new(0.0, omega(f) * self.driver.air_load_mass(&self.air))
    }

    /// Resuelve el sistema nodal a la frecuencia `f`.
    ///
    /// Devuelve `(presiones, Z_mouth, Z_driver_branch, p_s, u_driver)`.
    fn solve(
        &self,
        f: f64,
    ) -> Option<(Vec<Complex64>, Complex64, Complex64, Complex64, Complex64)> {
        let n = self.node_count();
        if n < 3 || self.tap_a == 0 || self.tap_b == 0 || self.tap_a >= n || self.tap_b >= n {
            return None;
        }
        let (p, zm, zd, ps) = self.solve_pressures(f, self.eg, None)?;
        let ia = self.tap_a - 1;
        let ib = self.tap_b - 1;
        let pa = p[ia];
        let pb = p[ib];
        let ud = (ps - (pa - pb)) / zd;
        let z_branch = if ud.norm() > 1.0e-30 {
            (pb - pa) / ud
        } else {
            Complex64::new(0.0, 0.0)
        };
        Some((p, zm, z_branch, ps, ud))
    }

    /// Resuelve las presiones nodales para una tensión `eg` y, opcionalmente,
    /// inyecta una corriente unidad en `test_node` para calcular una impedancia.
    fn solve_pressures(
        &self,
        f: f64,
        eg: f64,
        test_node: Option<usize>,
    ) -> Option<(Vec<Complex64>, Complex64, Complex64, Complex64)> {
        let n = self.node_count();
        if n < 3 || self.tap_a == 0 || self.tap_b == 0 || self.tap_a >= n || self.tap_b >= n {
            return None;
        }
        let zd_mech = self.driver_zm(f)
            + Complex64::new(self.driver.bl * self.driver.bl, 0.0)
                / (self.driver.coil_impedance(f) + Complex64::new(self.rg, 0.0));
        let zd = zd_mech / (self.driver.sd * self.driver.sd);
        let ze_coil = self.driver.coil_impedance(f) + Complex64::new(self.rg, 0.0);
        let ps =
            Complex64::new(self.driver.bl * eg / self.driver.sd, 0.0) / ze_coil * self.polarity;
        let zm = self.mouth_impedance(f);
        let ym = Complex64::new(1.0, 0.0) / zm;

        // Incógnitas: presiones de los nodos 1..n-1 (el nodo 0 = S1 está cerrado).
        let m = n - 1;
        let mut y = vec![vec![Complex64::new(0.0, 0.0); m]; m];
        let mut rhs = vec![Complex64::new(0.0, 0.0); m];

        for (i, seg) in self.segments.iter().enumerate() {
            let tr = seg.matrix(&self.air, f);
            if tr.b.norm() < 1.0e-30 {
                return None;
            }
            let y11 = -tr.a / tr.b;
            let y12 = Complex64::new(1.0, 0.0) / tr.b;
            let y22 = -tr.d / tr.b;
            if i == 0 {
                let y_stub = -tr.c / tr.a;
                y[0][0] += y_stub;
            } else {
                y[i - 1][i - 1] += y11;
                y[i - 1][i] += y12;
                y[i][i - 1] += y12;
                y[i][i] += y22;
            }
        }

        // Boca radiante (nodo n-1, última incógnita).
        y[n - 2][n - 2] += ym;

        // Driver entre las tomas a y b.
        let ia = self.tap_a - 1;
        let ib = self.tap_b - 1;
        let yd = Complex64::new(1.0, 0.0) / zd;
        y[ia][ia] += yd;
        y[ib][ib] += yd;
        y[ia][ib] -= yd;
        y[ib][ia] -= yd;
        let is = ps / zd;
        rhs[ia] += is;
        rhs[ib] -= is;

        if let Some(node) = test_node {
            if node >= 1 && node <= m {
                rhs[node - 1] += Complex64::new(1.0, 0.0);
            }
        }

        // Eliminación gaussiana con pivoteo parcial.
        for col in 0..m {
            let mut piv = col;
            for row in col + 1..m {
                if y[row][col].norm() > y[piv][col].norm() {
                    piv = row;
                }
            }
            if y[piv][col].norm() < 1.0e-30 {
                return None;
            }
            y.swap(col, piv);
            rhs.swap(col, piv);
            let d = y[col][col];
            for row in col + 1..m {
                let factor = y[row][col] / d;
                if factor.norm() == 0.0 {
                    continue;
                }
                // El índice k recorre dos filas de `y` a la vez; un iterador
                // no evita el préstamo mutable/immutable.
                #[allow(clippy::needless_range_loop)]
                for k in col..m {
                    let v = y[col][k] * factor;
                    y[row][k] -= v;
                }
                let v = rhs[col] * factor;
                rhs[row] -= v;
            }
        }
        let mut p = vec![Complex64::new(0.0, 0.0); m];
        for row in (0..m).rev() {
            let mut acc = rhs[row];
            for k in row + 1..m {
                acc -= y[row][k] * p[k];
            }
            p[row] = acc / y[row][row];
        }
        Some((p, zm, zd, ps))
    }

    /// Impedancia de carga del lado del cuello (driver retirado).
    ///
    /// Incluye el tramo cerrado S1→S2 y el camino S2→…→boca terminado en la
    /// radiación. Es la carga acústica del lado del cuello que muestra la
    /// gráfica de impedancia acústica de un tapped horn.
    pub fn neck_impedance(&self, f: f64) -> Complex64 {
        if self.segments.is_empty() || self.tap_a == 0 || self.tap_a > self.segments.len() {
            return Complex64::new(0.0, 0.0);
        }
        let tap = self.tap_a;
        let mut z_path = self.mouth_impedance(f);
        for seg in self.segments[tap..].iter().rev() {
            z_path = seg.matrix(&self.air, f).transform_impedance(z_path);
        }
        let y_path = Complex64::new(1.0, 0.0) / z_path;
        let tr0 = self.segments[0].matrix(&self.air, f);
        let y_stub = if tr0.a.norm() > 1.0e-30 {
            -tr0.c / tr0.a
        } else {
            Complex64::new(0.0, 0.0)
        };
        let y_total = y_stub + y_path;
        Complex64::new(1.0, 0.0) / y_total
    }

    /// Impedancia diferencial de la red pasiva entre las dos tomas, con el
    /// driver retirado. Inyecta una corriente unidad en `tap_a` y la extrae en
    /// `tap_b`; devuelve `p_a - p_b`.
    pub fn network_impedance(&self, f: f64) -> Complex64 {
        let n = self.node_count();
        if n < 3 || self.tap_a == 0 || self.tap_b == 0 || self.tap_a >= n || self.tap_b >= n {
            return Complex64::new(0.0, 0.0);
        }
        let m = n - 1;
        let mut y = vec![vec![Complex64::new(0.0, 0.0); m]; m];
        let mut rhs = vec![Complex64::new(0.0, 0.0); m];
        for (i, seg) in self.segments.iter().enumerate() {
            let tr = seg.matrix(&self.air, f);
            if tr.b.norm() < 1.0e-30 {
                return Complex64::new(0.0, 0.0);
            }
            if i == 0 {
                y[0][0] += -tr.c / tr.a;
            } else {
                y[i - 1][i - 1] += -tr.a / tr.b;
                y[i - 1][i] += Complex64::new(1.0, 0.0) / tr.b;
                y[i][i - 1] += Complex64::new(1.0, 0.0) / tr.b;
                y[i][i] += -tr.d / tr.b;
            }
        }
        y[n - 2][n - 2] += Complex64::new(1.0, 0.0) / self.mouth_impedance(f);
        let ia = self.tap_a - 1;
        let ib = self.tap_b - 1;
        rhs[ia] += Complex64::new(1.0, 0.0);
        rhs[ib] -= Complex64::new(1.0, 0.0);
        // Eliminación gaussiana.
        for col in 0..m {
            let mut piv = col;
            for row in col + 1..m {
                if y[row][col].norm() > y[piv][col].norm() {
                    piv = row;
                }
            }
            if y[piv][col].norm() < 1.0e-30 {
                return Complex64::new(0.0, 0.0);
            }
            y.swap(col, piv);
            rhs.swap(col, piv);
            let d = y[col][col];
            for row in col + 1..m {
                let factor = y[row][col] / d;
                // El índice k recorre dos filas de `y` a la vez; un iterador
                // no evita el préstamo mutable/immutable.
                #[allow(clippy::needless_range_loop)]
                for k in col..m {
                    let v = y[col][k] * factor;
                    y[row][k] -= v;
                }
                let v = rhs[col] * factor;
                rhs[row] -= v;
            }
        }
        let mut p = vec![Complex64::new(0.0, 0.0); m];
        for row in (0..m).rev() {
            let mut acc = rhs[row];
            for k in row + 1..m {
                acc -= y[row][k] * p[k];
            }
            p[row] = acc / y[row][row];
        }
        p[ia] - p[ib]
    }

    /// Impedancia en S2 con la rama del driver conectada y la fuente a cero.
    pub fn neck_impedance_with_driver(&self, f: f64) -> Complex64 {
        let ia = self.tap_a.saturating_sub(1);
        match self.solve_pressures(f, 0.0, Some(self.tap_a)) {
            Some((p, _, _, _)) if ia < p.len() => p[ia],
            _ => Complex64::new(0.0, 0.0),
        }
    }

    /// Presiones nodales del solve activo (para depuración).
    pub fn debug_pressures(&self, f: f64) -> Option<Vec<Complex64>> {
        self.solve_pressures(f, self.eg, None).map(|(p, _, _, _)| p)
    }

    /// Datos crudos para depuración: `(p_boca, Z_boca, u_driver, Z_rama, p_fuente)`.
    pub fn debug_at(
        &self,
        f: f64,
    ) -> Option<(Complex64, Complex64, Complex64, Complex64, Complex64)> {
        let (p, zm, zb, ps, ud) = self.solve(f)?;
        Some((p[self.node_count() - 2], zm, ud, zb, ps))
    }

    /// Respuesta a una frecuencia.
    fn zero_point(f: f64) -> TappedPoint {
        TappedPoint {
            freq: f,
            spl_db: f64::NEG_INFINITY,
            phase_deg: 0.0,
            ze_mag: 0.0,
            ze_phase_deg: 0.0,
            displacement_mm: 0.0,
            power_w: 0.0,
            efficiency_pct: 0.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn point_from_solve(
        &self,
        f: f64,
        p: &[Complex64],
        zm: Complex64,
        z_branch: Complex64,
        _ps: Complex64,
        ud: Complex64,
    ) -> TappedPoint {
        let n = self.node_count();
        let p_mouth = p[n - 2];
        let u_mouth = p_mouth / zm;
        let p_ac = u_mouth.norm_sqr() * zm.re * (std::f64::consts::TAU / self.solid_angle);
        let p_rms = if p_ac > 0.0 {
            (p_ac * self.air.rho * self.air.c / self.solid_angle).sqrt()
        } else {
            0.0
        };
        let spl = if p_rms > 0.0 {
            20.0 * (p_rms / self.air.p_ref).log10() + self.spl_offset_db
        } else {
            f64::NEG_INFINITY
        };

        let ze_coil = self.driver.coil_impedance(f) + Complex64::new(self.rg, 0.0);
        // Con la fuente de presión positiva en `tap_a`, z_branch = -Z_load.
        // La impedancia mecánica total es Zm + Sd²·Z_load.
        let ze = ze_coil
            + Complex64::new(self.driver.bl * self.driver.bl, 0.0)
                / (self.driver_zm(f) - self.driver.sd * self.driver.sd * z_branch);
        let v = ud / self.driver.sd;
        let current =
            (Complex64::new(self.eg, 0.0) - Complex64::new(self.driver.bl, 0.0) * v) / ze_coil;
        let p_in = self.eg * current.re;
        let disp_mm = (ud.norm() / (omega(f) * self.driver.sd)) * 1000.0 * std::f64::consts::SQRT_2;

        TappedPoint {
            freq: f,
            spl_db: spl,
            phase_deg: u_mouth.arg().to_degrees(),
            ze_mag: ze.norm(),
            ze_phase_deg: ze.arg().to_degrees(),
            displacement_mm: disp_mm,
            power_w: p_ac,
            efficiency_pct: if p_in > 0.0 { 100.0 * p_ac / p_in } else { 0.0 },
        }
    }

    /// Respuesta a una frecuencia. Devuelve un punto neutro si la entrada o el
    /// sistema no son resolubles; use [`TappedHorn::try_response_at`] para
    /// distinguir el error.
    pub fn response_at(&self, f: f64) -> TappedPoint {
        self.try_response_at(f)
            .unwrap_or_else(|_| Self::zero_point(f))
    }

    /// Respuesta a una frecuencia, con error explícito si la entrada, la
    /// configuración o el resultado no son válidos.
    pub fn try_response_at(&self, f: f64) -> Result<TappedPoint, SolveError> {
        check_frequency(f)?;
        self.validate()?;
        let Some((p, zm, z_branch, ps, ud)) = self.solve(f) else {
            return Err(SolveError::SingularSystem);
        };
        let point = self.point_from_solve(f, &p, zm, z_branch, ps, ud);
        point.validate()?;
        Ok(point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::Flare;

    fn driver() -> Driver {
        Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
    }

    fn th() -> TappedHorn {
        TappedHorn::new_4seg_th(
            driver(),
            vec![
                Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
                Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
                Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
                Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
            ],
        )
    }

    #[test]
    fn try_response_at_rejects_invalid_frequency() {
        let horn = th();
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                horn.try_response_at(f),
                Err(SolveError::InvalidFrequency(_))
            ));
        }
        assert!(horn.try_response_at(50.0).is_ok());
    }

    #[test]
    fn try_response_at_rejects_invalid_taps() {
        let mut horn = th();
        horn.tap_a = 0;
        assert!(matches!(
            horn.try_response_at(50.0),
            Err(SolveError::InvalidTopology(_))
        ));

        let mut horn = th();
        horn.tap_b = horn.tap_a;
        assert!(matches!(
            horn.try_response_at(50.0),
            Err(SolveError::InvalidTopology(_))
        ));

        let mut horn = th();
        horn.tap_b = 99;
        assert!(matches!(
            horn.try_response_at(50.0),
            Err(SolveError::InvalidTopology(_))
        ));
    }

    #[test]
    fn try_response_at_rejects_invalid_segment() {
        let mut horn = th();
        horn.segments[0] = Segment::new(Flare::Conical, 0.40, 0.0, 400e-4);
        assert!(matches!(
            horn.try_response_at(50.0),
            Err(SolveError::InvalidParameter("segment.s1"))
        ));
    }

    #[test]
    fn point_validation_rejects_invalid_fields() {
        let valid = th().response_at(50.0);
        assert!(valid.validate().is_ok());

        let mut p = valid;
        p.ze_mag = f64::NAN;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.displacement_mm = -1.0;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.power_w = f64::INFINITY;
        assert!(p.validate().is_err());
    }

    #[test]
    fn response_at_returns_neutral_point_on_invalid_input() {
        let p = th().response_at(f64::NAN);
        assert_eq!(p.spl_db, f64::NEG_INFINITY);
        assert_eq!(p.ze_mag, 0.0);
    }
}
