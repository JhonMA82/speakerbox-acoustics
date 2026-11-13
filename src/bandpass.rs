//! Recintos bandpass de 4º y 6º orden.
//!
//! - **BP4**: cámara trasera sellada + cámara delantera ventilada. La salida
//!   es el puerto delantero.
//! - **BP6**: cámara trasera ventilada + cámara delantera ventilada. Las dos
//!   salidas se suman en fase compleja.
//!
//! El diafragma ve `Z_carga = Z_trasera + Z_delantera`, donde cada cámara es
//! una compliancia en paralelo con su puerto. Por defecto el puerto se modela
//! como **tubo cilíndrico** (`PortModel::Pipe`), con su resonancia de cuarto de
//! onda y la radiación en la boca; `PortModel::Lumped` deja la masa acústica
//! simple `jωM`. La salida se calcula como presión lejana de un
//! volumen-velocidad en un ángulo sólido `Ω`:
//!
//! ```text
//! p(1 m) = ρ·jω·U_total / Ω
//! ```
//!
//! El desplazamiento se reporta de **pico** (`√2` veces el valor RMS).
//!
//! El BP6 puede ser **paralelo** (ambos puertos radian al exterior) o
//! **serie** (el puerto trasero desemboca en la cámara delantera y solo el
//! puerto delantero radia). La topología se selecciona con
//! [`RearPortRouting`]. En serie, el puerto trasero se resuelve con su matriz
//! de transmisión y la admitancia de ambas cámaras; su salida interior añade
//! la corrección de extremo embridada `0.8216·a`. El `QL` de la aplicación
//! de referencia se aplica solo a la cámara delantera.
//!
//! Validación: BP4, BP6 paralelo y BP6 serie contra exports propios del
//! software de referencia 60.70 (`tests/bp_production.rs`, 10 Hz–1 kHz).
//!
//! Referencias: Fincham, *A Bandpass Filter Loudspeaker System* (AES 1983);
//! Geddes, *An Introduction to Band-Pass Loudspeaker Systems* (JAES 1989).

use num_complex::Complex64;

use crate::air::Air;
use crate::driver::Driver;
use crate::element::{parallel, EndCorrection, Port};
use crate::error::{
    check_frequency, require_finite, require_non_negative, require_polarity, require_positive,
    SolveError,
};
use crate::omega;
use crate::radiation::piston_radiation_impedance;
use crate::segment::{Flare, Segment, TransferMatrix};

/// Orden del recinto bandpass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandpassOrder {
    /// 4º orden: trasera sellada, delantera ventilada.
    Fourth,
    /// 6º orden: trasera y delantera ventiladas.
    Sixth,
}

/// Destino de salida del puerto trasero en un BP6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RearPortRouting {
    /// El puerto trasero radia al exterior (BP6 paralelo).
    #[default]
    Outside,
    /// El puerto trasero descarga en la cámara delantera (BP6 serie).
    FrontChamber,
}

/// Modelo acústico del puerto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PortModel {
    /// Masa acústica lumped: `Z = jωM`, con corrección de extremo.
    Lumped,
    /// Tubo cilíndrico con su resonancia y radiación en la boca.
    #[default]
    Pipe,
}

/// Punto de respuesta de un bandpass.
#[derive(Clone, Copy, Debug)]
pub struct BandpassPoint {
    /// Frecuencia (Hz).
    pub freq: f64,
    /// SPL de potencia a 1 m (dB re 20 µPa).
    pub spl_db: f64,
    /// Fase reducida de presión observada estimada (grados); +90° da presión.
    /// NaN en silencio: la fase no está definida.
    pub phase_deg: f64,
    /// Módulo de la impedancia eléctrica (Ω).
    pub ze_mag: f64,
    /// Fase de la impedancia eléctrica (grados).
    pub ze_phase_deg: f64,
    /// Desplazamiento de pico del diafragma (mm).
    pub displacement_mm: f64,
    /// Potencia activa absorbida por las terminaciones explícitas (W).
    /// Puede ser un subtotal: consultar try_energy_at para calidad/radiación.
    /// Nunca se deduce de SPL ni incluye pérdidas mecánicas de un PR.
    pub power_w: f64,
    /// 100·potencia de terminaciones / potencia activa de la fuente (incluye Rg).
    /// No es η radiativa si el modelo está incompleto; NaN indica 0/0.
    /// try_energy_at distingue la potencia radiada disponible o aproximada.
    pub efficiency_pct: f64,
    /// Velocidad del aire en el puerto delantero (m/s).
    pub front_port_velocity: f64,
    /// Velocidad del aire en el puerto trasero (m/s), solo BP6.
    pub rear_port_velocity: Option<f64>,
}

/// Solución compleja RMS. En BP6 serie el caudal trasero es interior;
/// solamente `front_volume_velocity` radia al exterior.
#[derive(Clone, Copy, Debug)]
pub struct BandpassSolution {
    pub point: BandpassPoint,
    pub driver_volume_velocity: Complex64,
    pub front_volume_velocity: Complex64,
    pub rear_volume_velocity: Option<Complex64>,
    pub reduced_pressure_pa: Complex64,
    pub input_impedance: Complex64,
    pub input_current_a: Complex64,
    /// Potencia real absorbida por la red acústica, sin escalado de campo.
    pub acoustic_network_power_w: f64,
    /// Potencia activa suministrada, incluyendo Rg.
    pub electrical_input_power_w: f64,
}

impl BandpassPoint {
    /// Comprueba que el punto sea físicamente interpretable.
    pub fn validate(&self) -> Result<(), SolveError> {
        if !self.freq.is_finite() || self.freq <= 0.0 {
            return Err(SolveError::InvalidResponse("freq"));
        }
        if !(self.spl_db.is_finite() || self.spl_db == f64::NEG_INFINITY) {
            return Err(SolveError::InvalidResponse("spl_db"));
        }
        if !(self.phase_deg.is_finite()
            || (self.phase_deg.is_nan() && self.spl_db == f64::NEG_INFINITY))
        {
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
        if !((self.efficiency_pct.is_finite() && self.efficiency_pct >= 0.0)
            || (self.efficiency_pct.is_nan() && self.power_w == 0.0))
        {
            return Err(SolveError::InvalidResponse("efficiency_pct"));
        }
        if !(self.front_port_velocity.is_finite() && self.front_port_velocity >= 0.0) {
            return Err(SolveError::InvalidResponse("front_port_velocity"));
        }
        if let Some(velocity) = self.rear_port_velocity {
            if !(velocity.is_finite() && velocity >= 0.0) {
                return Err(SolveError::InvalidResponse("rear_port_velocity"));
            }
        }
        Ok(())
    }
}

/// Recinto bandpass de 4º o 6º orden.
#[derive(Clone, Debug)]
pub struct Bandpass {
    /// Propiedades del aire.
    pub air: Air,
    /// Altavoz.
    pub driver: Driver,
    /// Orden del bandpass (cuarto o sexto).
    pub order: BandpassOrder,
    /// Volumen interno de la cámara trasera (m³).
    pub rear_volume_m3: f64,
    /// Volumen interno de la cámara delantera (m³).
    pub front_volume_m3: f64,
    /// Puerto delantero.
    pub front_port: Port,
    /// Puerto trasero; obligatorio para [`BandpassOrder::Sixth`] y debe ser
    /// `None` en [`BandpassOrder::Fourth`]. `validate` rechaza la combinación
    /// contraria.
    pub rear_port: Option<Port>,
    /// Destino del puerto trasero: exterior (paralelo) o cámara delantera
    /// (serie). Solo se usa en BP6.
    pub rear_port_routing: RearPortRouting,
    /// Modelo de puerto usado para ambas cámaras.
    pub port_model: PortModel,
    /// Factor Ql de la cámara trasera (`None` = sin pérdidas).
    pub rear_ql: Option<f64>,
    /// Factor Ql de la cámara delantera (`None` = sin pérdidas).
    pub front_ql: Option<f64>,
    /// Ángulo sólido de radiación (estereorradianes).
    pub solid_angle: f64,
    /// Tensión de entrada (V); el signo invierte la polaridad.
    pub eg: f64,
    /// Resistencia de fuente (Ω).
    pub rg: f64,
    /// Corrección global de SPL (dB).
    pub spl_offset_db: f64,
    /// Polaridad del driver (+1 o -1).
    pub polarity: f64,
}

impl Bandpass {
    /// BP4: cámara trasera sellada, cámara delantera ventilada.
    pub fn new_bp4(
        driver: Driver,
        rear_volume_m3: f64,
        front_volume_m3: f64,
        front_port: Port,
    ) -> Self {
        Self {
            air: Air::default(),
            driver,
            order: BandpassOrder::Fourth,
            rear_volume_m3,
            front_volume_m3,
            front_port,
            rear_port: None,
            rear_port_routing: RearPortRouting::Outside,
            port_model: PortModel::Pipe,
            rear_ql: None,
            front_ql: None,
            solid_angle: std::f64::consts::TAU,
            eg: 2.83,
            rg: 0.0,
            spl_offset_db: 0.0,
            polarity: 1.0,
        }
    }

    /// BP6 paralelo: cámara trasera ventilada al exterior y cámara delantera
    /// ventilada al exterior.
    pub fn new_bp6(
        driver: Driver,
        rear_volume_m3: f64,
        front_volume_m3: f64,
        front_port: Port,
        rear_port: Port,
    ) -> Self {
        Self {
            air: Air::default(),
            driver,
            order: BandpassOrder::Sixth,
            rear_volume_m3,
            front_volume_m3,
            front_port,
            rear_port: Some(rear_port),
            rear_port_routing: RearPortRouting::Outside,
            port_model: PortModel::Pipe,
            rear_ql: None,
            front_ql: None,
            solid_angle: std::f64::consts::TAU,
            eg: 2.83,
            rg: 0.0,
            spl_offset_db: 0.0,
            polarity: 1.0,
        }
    }

    /// BP6 serie: el puerto trasero descarga en la cámara delantera y solo el
    /// puerto delantero radia al exterior.
    pub fn new_bp6_series(
        driver: Driver,
        rear_volume_m3: f64,
        front_volume_m3: f64,
        front_port: Port,
        rear_port: Port,
    ) -> Self {
        let mut bp = Self::new_bp6(
            driver,
            rear_volume_m3,
            front_volume_m3,
            front_port,
            rear_port,
        );
        bp.rear_port_routing = RearPortRouting::FrontChamber;
        bp
    }

    /// Frecuencia de sintonía del puerto delantero (Hz).
    pub fn front_tuning_hz(&self) -> f64 {
        tuning_hz(&self.air, self.front_port, self.front_volume_m3)
    }

    /// Comprueba la configuración del bandpass antes de calcular.
    ///
    /// Valida aire, driver, volúmenes, puerto delantero, puerto trasero
    /// obligatorio en BP6, `QL` y factores de calibración.
    pub fn validate(&self) -> Result<(), SolveError> {
        self.air.validate()?;
        self.driver.validate()?;
        require_positive(self.rear_volume_m3, "rear_volume_m3")?;
        require_positive(self.front_volume_m3, "front_volume_m3")?;
        self.front_port.validate()?;
        match self.order {
            BandpassOrder::Fourth => {
                if self.rear_port.is_some() {
                    return Err(SolveError::InvalidTopology("BP4 no usa puerto trasero"));
                }
            }
            BandpassOrder::Sixth => {
                let rear = self.rear_port.ok_or(SolveError::InvalidTopology(
                    "BP6 requiere un puerto trasero",
                ))?;
                rear.validate()?;
            }
        }
        if let Some(ql) = self.rear_ql {
            require_positive(ql, "rear_ql")?;
        }
        if let Some(ql) = self.front_ql {
            require_positive(ql, "front_ql")?;
        }
        require_positive(self.solid_angle, "solid_angle")?;
        require_finite(self.eg, "eg")?;
        require_non_negative(self.rg, "rg")?;
        require_finite(self.spl_offset_db, "spl_offset_db")?;
        require_polarity(self.polarity, "polarity")?;
        Ok(())
    }

    /// Frecuencia de sintonía del puerto trasero (Hz), si existe.
    pub fn rear_tuning_hz(&self) -> Option<f64> {
        self.rear_port
            .map(|port| tuning_hz(&self.air, port, self.rear_volume_m3))
    }

    /// Aplica el `QL` de un registro del software de referencia.
    ///
    /// Si `ql` no es finito o es `<= 0`, desactiva el `QL` de ambas cámaras
    /// (`None`); no se inventa un valor sustituto. El software de referencia
    /// guarda un solo `QL`, referido a la sintonía más baja. Para la
    /// otra cámara, el `QL` se escala con su frecuencia de sintonía para
    /// mantener la misma resistencia de pérdidas. En el BP6 serie se aplica
    /// solo a la cámara delantera, que es la que radia.
    pub fn set_ql(&mut self, ql: f64) {
        if !ql.is_finite() || ql <= 0.0 {
            self.front_ql = None;
            self.rear_ql = None;
            return;
        }
        match self.order {
            BandpassOrder::Fourth => {
                self.front_ql = Some(ql);
                self.rear_ql = None;
            }
            BandpassOrder::Sixth => {
                let fb_front = self.front_tuning_hz();
                let fb_rear = self.rear_tuning_hz().unwrap_or(fb_front);
                if self.rear_port_routing == RearPortRouting::FrontChamber {
                    // En BP6S el QL del software de referencia se refiere a la cámara que
                    // radia (la delantera); la trasera va acoplada por el
                    // puerto serie y no lleva una resistencia QL independiente.
                    self.rear_ql = None;
                    self.front_ql = if fb_front > 0.0 && fb_rear > 0.0 && fb_rear <= fb_front {
                        Some(ql * fb_front / fb_rear)
                    } else {
                        Some(ql)
                    };
                } else if fb_front <= 0.0 || fb_rear <= 0.0 {
                    self.front_ql = Some(ql);
                    self.rear_ql = Some(ql);
                } else if fb_rear <= fb_front {
                    self.rear_ql = Some(ql);
                    self.front_ql = Some(ql * fb_front / fb_rear);
                } else {
                    self.front_ql = Some(ql);
                    self.rear_ql = Some(ql * fb_rear / fb_front);
                }
            }
        }
    }

    fn rear_is_ported(&self) -> bool {
        self.order == BandpassOrder::Sixth
            && self
                .rear_port
                .map(|p| p.validate().is_ok())
                .unwrap_or(false)
    }

    fn chamber_impedance(&self, volume_m3: f64, f: f64, ql: Option<f64>, fb: f64) -> Complex64 {
        let c = volume_m3 / (self.air.rho * self.air.c * self.air.c);
        let z = Complex64::new(0.0, -1.0) / (omega(f) * c);
        match ql {
            Some(q) if q > 0.0 && fb > 0.0 => {
                // Resistencia en paralelo que da Ql a la frecuencia de sintonía.
                let r = q / (std::f64::consts::TAU * fb * c);
                parallel(z, Complex64::new(r, 0.0))
            }
            _ => z,
        }
    }

    /// Longitud efectiva del tubo del puerto: longitud física más la
    /// corrección del extremo interior. La radiación de la boca aporta la
    /// corrección exterior.
    fn pipe_length(port: Port) -> f64 {
        let a = (port.area / std::f64::consts::PI).sqrt();
        match port.end_correction {
            EndCorrection::None => port.length,
            EndCorrection::Inner(k) => port.length + k * a,
            EndCorrection::FlangedBoth | EndCorrection::UnflangedBoth => {
                port.length + port.end_correction.delta(port.area) * 0.5
            }
        }
    }

    fn port_input_impedance(&self, port: Port, f: f64) -> Complex64 {
        match self.port_model {
            PortModel::Lumped => port.impedance(&self.air, f),
            PortModel::Pipe => {
                let length = Self::pipe_length(port);
                let seg = Segment::new(Flare::Cylindrical, length, port.area, port.area);
                let z_rad = piston_radiation_impedance(&self.air, port.area, f, self.solid_angle);
                seg.matrix_validated(&self.air, f)
                    .transform_impedance(z_rad)
            }
        }
    }

    /// Matriz de transmisión del puerto, de la cámara de entrada a la salida.
    ///
    /// `extra_length` añade corrección de extremo en la salida; se usa en el
    /// puerto trasero del BP6 serie, cuyo extremo desemboca en la cámara
    /// delantera y no radica al exterior. Para `PortModel::Lumped` la
    /// longitud acústica es la misma que en Pipe: tramo interior + extra.
    /// Con exp(jωt), U positiva de entrada a salida, p2=p1-ZU:
    /// A=D=1, B=-jωρL/S, C=0.
    fn port_matrix(&self, port: Port, f: f64, extra_length: f64) -> TransferMatrix {
        match self.port_model {
            PortModel::Lumped => TransferMatrix {
                a: Complex64::new(1.0, 0.0),
                // p_out = p_in - Z·U; U positiva de entrada a salida.
                b: Complex64::new(
                    0.0,
                    -omega(f) * self.air.rho * (Self::pipe_length(port) + extra_length) / port.area,
                ),
                c: Complex64::new(0.0, 0.0),
                d: Complex64::new(1.0, 0.0),
            },
            PortModel::Pipe => {
                let length = Self::pipe_length(port) + extra_length;
                let seg = Segment::new(Flare::Cylindrical, length, port.area, port.area);
                seg.matrix_validated(&self.air, f)
            }
        }
    }

    /// Corrección de extremo embridado para una salida interior.
    fn flanged_end_correction(area: f64) -> f64 {
        0.8216 * (area / std::f64::consts::PI).sqrt()
    }

    fn port_out_velocity(&self, port: Port, p_chamber: Complex64, f: f64) -> Complex64 {
        match self.port_model {
            PortModel::Lumped => p_chamber / port.impedance(&self.air, f),
            PortModel::Pipe => {
                let length = Self::pipe_length(port);
                let seg = Segment::new(Flare::Cylindrical, length, port.area, port.area);
                let matrix = seg.matrix_validated(&self.air, f);
                let z_rad = piston_radiation_impedance(&self.air, port.area, f, self.solid_angle);
                let z_in = matrix.transform_impedance(z_rad);
                let u_in = p_chamber / z_in;
                let (_, u_out) = matrix.apply(p_chamber, u_in);
                u_out
            }
        }
    }

    /// Respuesta a una frecuencia. Devuelve un punto neutro si la topología no
    /// es válida; use [`Bandpass::try_response_at`] para distinguir el error.
    pub fn response_at(&self, f: f64) -> BandpassPoint {
        self.try_response_at(f)
            .unwrap_or_else(|_| self.zero_point(f))
    }

    /// Respuesta a una frecuencia, con error explícito si la entrada, la
    /// configuración o el resultado no son válidos.
    pub fn try_response_at(&self, f: f64) -> Result<BandpassPoint, SolveError> {
        check_frequency(f)?;
        self.validate()?;
        self.response_validated(f)
    }

    pub(crate) fn response_validated(&self, f: f64) -> Result<BandpassPoint, SolveError> {
        Ok(self.solution_validated(f)?.point)
    }
    /// Devuelve fasores y cada puerto sin reducirlos a un máximo.
    pub fn try_solution_at(&self, f: f64) -> Result<BandpassSolution, SolveError> {
        check_frequency(f)?;
        self.validate()?;
        self.solution_validated(f)
    }
    fn solution_validated(&self, f: f64) -> Result<BandpassSolution, SolveError> {
        let s = self.solve_point(f).ok_or(SolveError::SingularSystem)?;
        s.point.validate()?;
        for z in [
            Some(s.driver_volume_velocity),
            Some(s.front_volume_velocity),
            s.rear_volume_velocity,
            Some(s.reduced_pressure_pa),
            Some(s.input_impedance),
            Some(s.input_current_a),
        ]
        .into_iter()
        .flatten()
        {
            if !z.re.is_finite() || !z.im.is_finite() {
                return Err(SolveError::NonFiniteResponse);
            }
        }
        Ok(s)
    }

    /// Balance de potencia por cámaras y puertos. Solo los puertos exteriores
    /// de Pipe tienen carga de pistón; el puerto BP6 serie es interior.
    pub fn try_energy_at(&self, f: f64) -> Result<crate::energy::EnergyReport, SolveError> {
        use crate::energy::{
            load_termination, shunt_power, EnergyReport, RadiationQuality as Q,
            TerminationKind as K,
        };
        let s = self.try_solution_at(f)?;
        let mut report = EnergyReport::new(
            &self.driver,
            f,
            self.eg,
            self.rg,
            s.input_current_a,
            s.driver_volume_velocity,
            s.acoustic_network_power_w,
        );
        let chamber_pressure = |port: Port, u: Complex64| match self.port_model {
            PortModel::Lumped => port.impedance(&self.air, f) * u,
            PortModel::Pipe => {
                let m = self.port_matrix(port, f, 0.0);
                let z = piston_radiation_impedance(&self.air, port.area, f, self.solid_angle);
                (m.d * z * u - m.b * u) / (m.a * m.d - m.b * m.c)
            }
        };
        let pf = chamber_pressure(self.front_port, s.front_volume_velocity);
        let fb_f = self.front_tuning_hz();
        let fb_r = self.rear_tuning_hz().unwrap_or(fb_f);
        let zcr = self.chamber_impedance(self.rear_volume_m3, f, self.rear_ql, fb_r);
        let zcf = self.chamber_impedance(self.front_volume_m3, f, self.front_ql, fb_f);
        let pr = match (
            self.rear_port_routing,
            self.rear_port,
            s.rear_volume_velocity,
        ) {
            (RearPortRouting::FrontChamber, Some(port), Some(u)) => {
                let m = self.port_matrix(port, f, Self::flanged_end_correction(port.area));
                (m.d * pf - m.b * u) / (m.a * m.d - m.b * m.c)
            }
            (RearPortRouting::Outside, Some(port), Some(u)) => chamber_pressure(port, u),
            _ => -s.driver_volume_velocity * zcr,
        };
        report.acoustic_losses.chamber_ql_w = shunt_power(pf, zcf) + shunt_power(pr, zcr);
        if self.port_model == PortModel::Lumped {
            return report.finish(Q::IncompleteRadiationModel);
        }
        let zf = piston_radiation_impedance(&self.air, self.front_port.area, f, self.solid_angle);
        report.terminations.push(load_termination(
            K::BaffledPiston,
            s.front_volume_velocity,
            zf,
        ));
        let mut quality = Q::SingleBaffledPiston;
        if let (RearPortRouting::Outside, Some(port), Some(u)) = (
            self.rear_port_routing,
            self.rear_port,
            s.rear_volume_velocity,
        ) {
            let z = piston_radiation_impedance(&self.air, port.area, f, self.solid_angle);
            report
                .terminations
                .push(load_termination(K::BaffledPiston, u, z));
            quality = Q::IndependentLoadsApproximation;
        }
        report.finish(quality)
    }

    fn zero_point(&self, f: f64) -> BandpassPoint {
        BandpassPoint {
            freq: f,
            spl_db: f64::NEG_INFINITY,
            phase_deg: 0.0,
            ze_mag: 0.0,
            ze_phase_deg: 0.0,
            displacement_mm: 0.0,
            power_w: 0.0,
            efficiency_pct: 0.0,
            front_port_velocity: 0.0,
            rear_port_velocity: None,
        }
    }

    fn solve_point(&self, f: f64) -> Option<BandpassSolution> {
        let w = omega(f);
        let fb_front = self.front_tuning_hz();
        let fb_rear = self.rear_tuning_hz().unwrap_or(fb_front);

        let z_cr = self.chamber_impedance(self.rear_volume_m3, f, self.rear_ql, fb_rear);
        let z_cf = self.chamber_impedance(self.front_volume_m3, f, self.front_ql, fb_front);

        let z_pf = self.port_input_impedance(self.front_port, f);
        let z_front = parallel(z_cf, z_pf);

        let rear_port = if self.rear_is_ported() {
            self.rear_port
        } else {
            None
        };
        let z_pr = rear_port.map(|p| self.port_input_impedance(p, f));
        let z_rear = match z_pr {
            Some(z_pr) => parallel(z_cr, z_pr),
            None => z_cr,
        };

        // En el BP6 serie el puerto trasero conecta las dos cámaras. Se resuelve
        // la red de dos puertas con la matriz del puerto y las admitancias de
        // ambas cámaras; el puerto delantero sigue radiando al exterior.
        let one = Complex64::new(1.0, 0.0);
        let series = match (self.rear_port_routing, rear_port) {
            (RearPortRouting::FrontChamber, Some(port)) => {
                let m = self.port_matrix(port, f, Self::flanged_end_correction(port.area));
                let y_cr = one / z_cr;
                let y_f = one / z_front;
                let denom = (m.d * y_cr - m.c) + (m.a - m.b * y_cr) * y_f;
                if denom.norm() < 1.0e-30 {
                    return None;
                }
                let pf_u = (m.a - m.b * y_cr - one) / denom;
                let uout_u = y_f * pf_u - one;
                let pr_u = m.d * pf_u - m.b * uout_u;
                Some((pf_u - pr_u, pf_u, uout_u))
            }
            _ => None,
        };
        let z_load = series.map_or(z_rear + z_front, |s| s.0);
        let u_d = self
            .driver
            .volume_velocity(f, self.eg * self.polarity, self.rg, z_load);
        if !u_d.re.is_finite() || !u_d.im.is_finite() {
            return None;
        }

        // Presión de la cámara delantera y caudal del puerto trasero.
        let (p_front, u_pr) = match series {
            Some((_, pf_u, uout_u)) => (pf_u * u_d, Some(uout_u * u_d)),
            None => {
                let p_front = z_front * u_d;
                let p_rear = -u_d * z_rear;
                let u_pr = rear_port.map(|port| self.port_out_velocity(port, p_rear, f));
                (p_front, u_pr)
            }
        };

        let u_pf = self.port_out_velocity(self.front_port, p_front, f);
        let v_pf = (u_pf / self.front_port.area).norm();

        // Solo el puerto delantero radia en serie; en paralelo se suman ambos.
        let u_total = match (self.rear_port_routing, rear_port) {
            (RearPortRouting::Outside, Some(_)) => u_pf + u_pr.unwrap_or_default(),
            _ => u_pf,
        };
        let v_pr = match (rear_port, u_pr) {
            (Some(port), Some(u_pr)) => Some((u_pr / port.area).norm()),
            _ => None,
        };

        let p_rms = self.air.rho * w * u_total.norm() / self.solid_angle;
        let spl = if p_rms > 0.0 {
            20.0 * (p_rms / self.air.p_ref).log10() + self.spl_offset_db
        } else {
            f64::NEG_INFINITY
        };
        // Pipe es sin pérdidas: el flujo activo a la entrada es el de su
        // terminación. Reutilizar Z_pf/Z_pr evita recalcular Bessel/Struve.
        // Lumped solo modela una inertancia con salida de presión nula.
        let p_ac = if self.port_model == PortModel::Pipe {
            let front = p_front.norm_sqr() * (one / z_pf).re;
            front
                + match (self.rear_port_routing, z_pr) {
                    (RearPortRouting::Outside, Some(z)) => {
                        (-u_d * z_rear).norm_sqr() * (one / z).re
                    }
                    _ => 0.0,
                }
        } else {
            0.0
        };

        let ze_coil = self.driver.coil_impedance(f) + Complex64::new(self.rg, 0.0);
        let zm = self.driver.mechanical_impedance(f);
        let ze = ze_coil
            + Complex64::new(self.driver.bl * self.driver.bl, 0.0)
                / (zm + self.driver.sd * self.driver.sd * z_load);
        if !ze.re.is_finite() || !ze.im.is_finite() || ze.norm() <= 0.0 {
            return None;
        }
        let v = u_d / self.driver.sd;
        let current = Complex64::new(self.eg, 0.0) / ze;
        let p_in = self.eg * current.re;
        let disp_mm = (v.norm() / w) * 1000.0 * std::f64::consts::SQRT_2;

        let point = BandpassPoint {
            freq: f,
            spl_db: spl,
            phase_deg: if p_rms > 0.0 {
                u_total.arg().to_degrees()
            } else {
                f64::NAN
            },
            ze_mag: ze.norm(),
            ze_phase_deg: ze.arg().to_degrees(),
            displacement_mm: disp_mm,
            power_w: p_ac,
            efficiency_pct: if p_in > 0.0 {
                100.0 * p_ac / p_in
            } else {
                f64::NAN
            },
            front_port_velocity: v_pf,
            rear_port_velocity: v_pr,
        };
        Some(BandpassSolution {
            point,
            driver_volume_velocity: u_d,
            front_volume_velocity: u_pf,
            rear_volume_velocity: u_pr,
            reduced_pressure_pa: u_total * (self.air.rho * w / self.solid_angle),
            input_impedance: ze,
            input_current_a: current,
            acoustic_network_power_w: z_load.re * u_d.norm_sqr(),
            electrical_input_power_w: self.eg * current.re,
        })
    }

    /// Barrido de respuesta. Se eliminan las frecuencias no finitas o no
    /// positivas.
    pub fn response(&self, freqs: &[f64]) -> Vec<BandpassPoint> {
        freqs
            .iter()
            .copied()
            .filter(|f| f.is_finite() && *f > 0.0)
            .map(|f| self.response_at(f))
            .collect()
    }
}

fn tuning_hz(air: &Air, port: Port, volume_m3: f64) -> f64 {
    if port.area <= 0.0 || volume_m3 <= 0.0 {
        return 0.0;
    }
    let l_eff = port.effective_length();
    if l_eff <= 0.0 {
        return 0.0;
    }
    air.c / std::f64::consts::TAU * (port.area / (l_eff * volume_m3)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::EndCorrection;

    fn driver() -> Driver {
        Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
    }

    fn port(area_cm2: f64, length_cm: f64) -> Port {
        Port::new(
            area_cm2 * 1.0e-4,
            length_cm * 1.0e-2,
            EndCorrection::UnflangedBoth,
        )
    }

    fn bp4() -> Bandpass {
        Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, port(50.0, 10.0))
    }

    fn bp6() -> Bandpass {
        Bandpass::new_bp6(
            driver(),
            20.0e-3,
            10.0e-3,
            port(50.0, 10.0),
            port(40.0, 8.0),
        )
    }

    #[test]
    fn bp4_is_finite_and_has_a_passband() {
        let bp = bp4();
        for f in [10.0, 20.0, 50.0, 100.0, 500.0, 2000.0] {
            let p = bp.response_at(f);
            assert!(p.spl_db.is_finite(), "SPL no finito a {f} Hz");
            assert!(p.ze_mag.is_finite() && p.ze_mag > 0.0);
            assert!(p.displacement_mm.is_finite() && p.displacement_mm >= 0.0);
            assert!(p.front_port_velocity.is_finite());
        }
        assert!(bp.response_at(100.0).spl_db > bp.response_at(20.0).spl_db);
    }

    #[test]
    fn front_tuning_decreases_with_larger_volume() {
        let small = Bandpass::new_bp4(driver(), 20.0e-3, 5.0e-3, port(50.0, 10.0));
        let large = Bandpass::new_bp4(driver(), 20.0e-3, 20.0e-3, port(50.0, 10.0));
        assert!(large.front_tuning_hz() < small.front_tuning_hz());
    }

    #[test]
    fn bp6_requires_a_rear_port() {
        let mut bp = bp6();
        bp.rear_port = None;
        assert!(matches!(
            bp.try_response_at(100.0),
            Err(SolveError::InvalidTopology(_))
        ));
        assert!(bp.response_at(100.0).spl_db == f64::NEG_INFINITY);
    }

    #[test]
    fn bp6_differs_from_bp4() {
        let f = bp6().rear_tuning_hz().unwrap_or(70.0);
        let a = bp4().response_at(f).spl_db;
        let b = bp6().response_at(f).spl_db;
        assert!((a - b).abs() > 0.1, "f={f} Hz: BP4={a} dB, BP6={b} dB");
    }

    #[test]
    fn bp6_reports_both_port_velocities() {
        let p = bp6().response_at(100.0);
        assert!(p.front_port_velocity > 0.0);
        assert!(p.rear_port_velocity.unwrap_or(0.0) > 0.0);
    }

    fn bp6_series() -> Bandpass {
        Bandpass::new_bp6_series(
            driver(),
            20.0e-3,
            10.0e-3,
            port(50.0, 10.0),
            port(40.0, 8.0),
        )
    }

    #[test]
    fn bp6_series_is_finite_and_has_a_passband() {
        let bp = bp6_series();
        for f in [10.0, 20.0, 50.0, 100.0, 500.0, 2000.0] {
            let p = bp.response_at(f);
            assert!(p.spl_db.is_finite(), "SPL no finito a {f} Hz");
            assert!(p.ze_mag.is_finite() && p.ze_mag > 0.0);
            assert!(p.displacement_mm.is_finite() && p.displacement_mm >= 0.0);
            assert!(p.front_port_velocity.is_finite());
            assert!(p.rear_port_velocity.unwrap_or(0.0).is_finite());
        }
        assert!(bp.response_at(100.0).spl_db > bp.response_at(20.0).spl_db);
    }

    #[test]
    fn bp6_series_differs_from_parallel() {
        let f = 50.0;
        let a = bp6_series().response_at(f).spl_db;
        let b = bp6().response_at(f).spl_db;
        assert!((a - b).abs() > 1.0, "serie={a} dB, paralelo={b} dB");
    }

    #[test]
    fn bp6_series_ql_is_applied_to_the_front_chamber_only() {
        let mut bp = bp6_series();
        bp.set_ql(7.0);
        assert!(bp.rear_ql.is_none());
        let front = bp.front_ql.expect("QL delantero");
        assert!((front - 7.0 * bp.front_tuning_hz() / bp.rear_tuning_hz().unwrap()).abs() < 1.0e-9);
    }

    #[test]
    fn bp6_series_with_closed_rear_port_matches_bp4() {
        // Un puerto trasero casi cerrado elimina la realimentación al frontal;
        // el BP6 serie debe converger al BP4 equivalente.
        let mut series = Bandpass::new_bp6_series(
            driver(),
            20.0e-3,
            10.0e-3,
            port(50.0, 10.0),
            port(1.0e-4, 1.0),
        );
        series.port_model = PortModel::Pipe;
        let bp4 = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, port(50.0, 10.0));
        for f in [20.0, 50.0, 100.0, 200.0] {
            let a = series.response_at(f).spl_db;
            let b = bp4.response_at(f).spl_db;
            assert!((a - b).abs() < 0.05, "f={f}: serie={a} dB, BP4={b} dB");
        }
    }

    #[test]
    fn pipe_and_lumped_differ_near_port_resonance() {
        let p = port(50.0, 30.0);
        let mut pipe = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, p);
        pipe.port_model = PortModel::Pipe;
        let mut lumped = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, p);
        lumped.port_model = PortModel::Lumped;
        let a = pipe.response_at(450.0).spl_db;
        let b = lumped.response_at(450.0).spl_db;
        assert!((a - b).abs() > 1.0, "pipe={a} dB, lumped={b} dB");
    }

    #[test]
    fn pipe_low_frequency_matches_lumped() {
        let p = port(50.0, 10.0);
        let mut pipe = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, p);
        pipe.port_model = PortModel::Pipe;
        let mut lumped = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, p);
        lumped.port_model = PortModel::Lumped;
        let a = pipe.response_at(10.0);
        let b = lumped.response_at(10.0);
        assert!((a.spl_db - b.spl_db).abs() < 1.0);
        assert!((a.ze_mag - b.ze_mag).abs() / b.ze_mag < 0.05);
    }

    #[test]
    fn pipe_response_is_finite() {
        let mut bp = bp4();
        bp.port_model = PortModel::Pipe;
        for f in [10.0, 20.0, 50.0, 100.0, 500.0, 1000.0, 2000.0] {
            let p = bp.response_at(f);
            assert!(p.spl_db.is_finite(), "SPL no finito a {f} Hz");
            assert!(p.ze_mag.is_finite() && p.ze_mag > 0.0);
            assert!(p.front_port_velocity.is_finite());
        }
    }

    #[test]
    fn bp4_reference_excursion_is_peak() {
        let mut bp = Bandpass::new_bp4(
            Driver::from_ui_units(211.20, 5.91e-4, 31.43, 5.90, 10.65, 4.11, 1.0, 1),
            12.0e-3,
            15.0e-3,
            port(std::f64::consts::PI * 3.5 * 3.5, 15.0),
        );
        bp.front_ql = Some(7.0);
        // Referencia 60.70, mismo diseño: Xd = 1.043 mm a 10 Hz (pico).
        let xd = bp.response_at(10.0).displacement_mm;
        assert!((xd - 1.043).abs() < 0.02, "Xd={xd} mm");
    }

    fn reference_driver() -> Driver {
        Driver::from_ui_units(211.20, 5.91e-4, 31.43, 5.90, 10.65, 4.11, 1.0, 1)
    }

    fn round_port(d_cm: f64, l_cm: f64) -> Port {
        let d = d_cm * 1.0e-2;
        Port::new(
            std::f64::consts::PI * d * d / 4.0,
            l_cm * 1.0e-2,
            EndCorrection::UnflangedBoth,
        )
    }

    #[test]
    fn bp4_matches_reference_points() {
        let mut bp = Bandpass::new_bp4(reference_driver(), 12.0e-3, 15.0e-3, round_port(7.0, 15.0));
        bp.front_ql = Some(7.0);
        // Puntos de referencia 60.70 (Ang = 2π, Eg = 2.83 V).
        let refs = [
            (10.0, 55.65, 6.04, 1.043),
            (50.33, 88.93, 11.36, 0.672),
            (100.0, 83.98, 25.21, 0.459),
            (198.67, 72.44, 6.88, 0.164),
            (503.34, 59.20, 6.27, 0.022),
            (1000.0, 57.38, 8.22, 0.004),
        ];
        for (f, spl, ze, xd) in refs {
            let p = bp.response_at(f);
            assert!(
                (p.spl_db - spl).abs() <= 1.0,
                "SPL {f} Hz: {:.2} vs {spl}",
                p.spl_db
            );
            assert!(
                (p.ze_mag - ze).abs() / ze <= 0.02,
                "Ze {f} Hz: {:.2} vs {ze}",
                p.ze_mag
            );
            if xd > 0.02 {
                assert!(
                    (p.displacement_mm - xd).abs() / xd <= 0.05,
                    "Xd {f} Hz: {:.4} vs {xd}",
                    p.displacement_mm
                );
            }
        }
    }

    #[test]
    fn bp6_parallel_matches_reference_points() {
        let mut bp = Bandpass::new_bp6(
            reference_driver(),
            20.0e-3,
            15.0e-3,
            round_port(7.0, 15.0),
            round_port(6.0, 20.0),
        );
        bp.rear_ql = Some(7.0);
        bp.front_ql = Some(10.38);
        let refs = [
            (10.0, 38.48, 8.93, 3.709),
            (19.87, 60.06, 27.34, 2.395),
            (50.33, 91.93, 11.35, 0.576),
            (100.0, 80.39, 26.20, 0.465),
            (198.67, 66.91, 6.77, 0.158),
            (503.34, 49.29, 6.27, 0.022),
            (1000.0, 54.82, 8.22, 0.004),
        ];
        for (f, spl, ze, xd) in refs {
            let p = bp.response_at(f);
            assert!(
                (p.spl_db - spl).abs() <= 1.0,
                "SPL {f} Hz: {:.2} vs {spl}",
                p.spl_db
            );
            assert!(
                (p.ze_mag - ze).abs() / ze <= 0.02,
                "Ze {f} Hz: {:.2} vs {ze}",
                p.ze_mag
            );
            if xd > 0.02 {
                assert!(
                    (p.displacement_mm - xd).abs() / xd <= 0.05,
                    "Xd {f} Hz: {:.4} vs {xd}",
                    p.displacement_mm
                );
            }
        }
    }

    #[test]
    fn invalid_volumes_are_rejected() {
        let mut bp = bp4();
        bp.front_volume_m3 = 0.0;
        assert!(matches!(
            bp.try_response_at(100.0),
            Err(SolveError::InvalidParameter("front_volume_m3"))
        ));
    }

    #[test]
    fn try_response_at_rejects_invalid_frequency() {
        let bp = bp4();
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                bp.try_response_at(f),
                Err(SolveError::InvalidFrequency(_))
            ));
        }
        assert!(bp.try_response_at(100.0).is_ok());
    }

    #[test]
    fn bp4_rejects_a_rear_port() {
        let mut bp = bp4();
        bp.rear_port = Some(port(40.0, 8.0));
        assert!(matches!(
            bp.try_response_at(100.0),
            Err(SolveError::InvalidTopology(_))
        ));
    }

    #[test]
    fn invalid_ql_is_rejected() {
        let mut bp = bp4();
        bp.front_ql = Some(0.0);
        assert!(matches!(
            bp.try_response_at(100.0),
            Err(SolveError::InvalidParameter("front_ql"))
        ));
    }

    #[test]
    fn point_validation_rejects_invalid_fields() {
        let valid = bp4().response_at(100.0);
        assert!(valid.validate().is_ok());

        let mut p = valid;
        p.ze_mag = f64::NAN;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.front_port_velocity = -1.0;
        assert!(p.validate().is_err());

        let mut p = valid;
        p.rear_port_velocity = Some(f64::INFINITY);
        assert!(p.validate().is_err());
    }

    #[test]
    fn response_filters_non_finite_frequencies() {
        let pts = bp4().response(&[f64::NAN, f64::INFINITY, 0.0, -1.0, 100.0]);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].freq - 100.0).abs() < 1.0e-12);
    }
}
