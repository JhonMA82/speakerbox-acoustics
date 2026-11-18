//! Constructores de redes distribuidas. Los parámetros geométricos están en SI.
use super::{
    AcousticBoundary, AcousticDuct, AcousticNetwork, AcousticNode, Boundary, Conduit, NetworkError,
};
use crate::{Flare, Port, Segment};

/// Red y asignación de las dos caras del diafragma.
#[derive(Clone, Debug)]
pub struct NetworkLayout {
    pub network: AcousticNetwork,
    /// Nodo que recibe el caudal positivo del diafragma.
    pub front: AcousticNode,
    /// Nodo que recibe el caudal opuesto.
    pub rear: AcousticNode,
}

/// Cámara cilíndrica uniforme. Volumen y longitud positivos fijan el área V/L.
#[derive(Clone, Copy, Debug)]
pub struct DistributedChamber {
    pub volume_m3: f64,
    pub length_m: f64,
    /// Resistencia de fuga medida o declarada, conectada a la entrada.
    pub leakage_pa_s_m3: Option<f64>,
}

impl DistributedChamber {
    fn segment(self) -> Result<Segment, NetworkError> {
        crate::error::require_positive(self.volume_m3, "chamber volume")?;
        crate::error::require_positive(self.length_m, "chamber length")?;
        if let Some(r) = self.leakage_pa_s_m3 {
            crate::error::require_positive(r, "chamber leakage resistance")?;
        }
        cylinder(self.length_m, self.volume_m3 / self.length_m)
    }
}

impl NetworkLayout {
    fn empty(front: AcousticNode, rear: AcousticNode, nodes: usize) -> Self {
        Self {
            network: AcousticNetwork {
                nodes,
                ..AcousticNetwork::default()
            },
            front,
            rear,
        }
    }

    fn duct(&mut self, a: AcousticNode, b: AcousticNode, segment: Segment) {
        self.network.ducts.push(AcousticDuct {
            inlet: a,
            outlet: b,
            conduit: Conduit::Distributed(segment),
        });
    }

    fn shunt(&mut self, node: AcousticNode, boundary: Boundary) {
        self.network
            .boundaries
            .push(AcousticBoundary { node, boundary });
    }

    fn mouth(&mut self, node: AcousticNode, area_m2: f64) {
        self.shunt(
            node,
            Boundary::Opening {
                area_m2,
                load_scale: 1.0,
            },
        );
    }

    fn chamber(
        &mut self,
        inlet: AcousticNode,
        outlet: AcousticNode,
        chamber: DistributedChamber,
    ) -> Result<(), NetworkError> {
        self.duct(inlet, outlet, chamber.segment()?);
        if let Some(pa_s_m3) = chamber.leakage_pa_s_m3 {
            self.shunt(inlet, Boundary::Resistance { pa_s_m3 });
        }
        Ok(())
    }

    fn port(&mut self, a: AcousticNode, b: AcousticNode, port: Port) -> Result<(), NetworkError> {
        port.validate()?;
        self.duct(a, b, cylinder(port.effective_length(), port.area)?);
        Ok(())
    }

    /// PH1. H1 y H5 llegan desde sus extremos cerrados hasta el diafragma.
    /// H2/H3 y H6/H7 unen las dos caras al colector del tramo H4.
    pub fn paraflex_ph1(lower: [Segment; 4], upper: [Segment; 3]) -> Result<Self, NetworkError> {
        let mut layout = Self::empty(1, 6, 8);
        let mouth_area = lower[3].s2();
        for (segment, (a, b)) in lower.into_iter().chain(upper).zip([
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (5, 6),
            (6, 7),
            (7, 3),
        ]) {
            segment.validate()?;
            layout.duct(a, b, segment);
        }
        layout.mouth(4, mouth_area);
        Ok(layout)
    }

    /// DBR. El cono frontal radia directamente, la cara posterior alimenta C1.
    pub fn double_bass_reflex(
        sd: f64,
        chambers: [DistributedChamber; 2],
        ports: [Port; 2],
    ) -> Result<Self, NetworkError> {
        crate::error::require_positive(sd, "DBR Sd")?;
        let mut layout = Self::empty(0, 1, 6);
        layout.chamber(1, 2, chambers[0])?;
        layout.chamber(3, 4, chambers[1])?;
        layout.port(2, 3, ports[0])?;
        layout.port(4, 5, ports[1])?;
        layout.mouth(0, sd);
        layout.mouth(5, ports[1].area);
        Ok(layout)
    }

    /// BP8S. C2 → C1 → C3 → exterior. Las caras del cono están en C1 y C2.
    pub fn bandpass_eighth_series(
        chambers: [DistributedChamber; 3],
        ports: [Port; 3],
    ) -> Result<Self, NetworkError> {
        let mut layout = Self::three_chambers(chambers)?;
        layout.port(1, 4, ports[0])?;
        layout.port(3, 0, ports[1])?;
        layout.port(5, 6, ports[2])?;
        layout.mouth(6, ports[2].area);
        Ok(layout)
    }

    /// BPB. C2 queda cerrada. C1 → C3 → exterior.
    pub fn bandpass_b(
        chambers: [DistributedChamber; 3],
        inner_port: Port,
        mouth_port: Port,
    ) -> Result<Self, NetworkError> {
        let mut layout = Self::three_chambers(chambers)?;
        layout.port(1, 4, inner_port)?;
        layout.port(5, 6, mouth_port)?;
        layout.mouth(6, mouth_port.area);
        Ok(layout)
    }

    fn three_chambers(chambers: [DistributedChamber; 3]) -> Result<Self, NetworkError> {
        let mut layout = Self::empty(0, 2, 7);
        for (i, chamber) in chambers.into_iter().enumerate() {
            layout.chamber(2 * i, 2 * i + 1, chamber)?;
        }
        Ok(layout)
    }
}

fn cylinder(length: f64, area: f64) -> Result<Segment, NetworkError> {
    let seg = Segment::new(Flare::Cylindrical, length, area, area);
    seg.validate()?;
    Ok(seg)
}
