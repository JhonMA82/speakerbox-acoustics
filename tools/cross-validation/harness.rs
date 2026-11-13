// Original comparison harness; no third-party implementation copied.
use speakerbox_acoustics::{Air,Driver,System,Chamber,Design};
use loudspeaker_solver::{types::DriverParams,types::SealedBoxParams,driver::derive_driver,sealed::sealed_frequency_response};
fn main(){
 let output=std::env::args().nth(1).expect("output directory");
 let p=DriverParams{fs_hz:30.,re_ohm:6.,le_h:0.001,qes:0.4,qms:5.,vas_m3:0.08,sd_m2:0.035,xmax_m:0.01,ke:0.};
 let d=derive_driver(&p);let air=Air::new(343.21,1.2041);
 let mut ours=Driver::new(p.sd_m2,d.cms,d.mms,p.re_ohm,d.bl,d.rms,p.le_h);
 // Reference Mms already includes low-frequency piston air inertia.
 ours.mmd-=ours.air_load_mass(&air);
 let mut sys=System::new(air,ours);sys.eg=2.83;sys.rg=0.;sys.rear_chamber=Some(Chamber::new(0.04));
 let design=Design::from(sys);let freqs:Vec<_>=(0..201).map(|i|5.*20_f64.powf(i as f64/200.)).collect();
 let reference=sealed_frequency_response(&d,&SealedBoxParams{volume_m3:0.04,ql:1e9},&freqs,2.83);
 let points=design.response(&freqs);
 let mut a=String::from("frequency_hz,spl_db,ze_ohm,ze_phase_deg,excursion_peak_mm,phase_deg,delay_ms\n");let mut b=a.clone();
 for (i,point) in points.iter().enumerate(){
 a+=&format!("{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",freqs[i],point.spl_db,point.ze_mag,point.ze_phase_deg,point.xd_mm,point.phase_deg+90.,point.group_delay_ms);
 // Reference implementation returns RMS displacement despite 'peak' field docs.
 b+=&format!("{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",freqs[i],reference.spl_db[i],reference.impedance_ohm[i],reference.impedance_phase_deg[i],reference.cone_displacement_mm[i]*2_f64.sqrt(),reference.acoustic_phase_deg[i],reference.group_delay_ms[i]);
 }
 std::fs::write(format!("{output}/speakerbox.csv"),a).unwrap();std::fs::write(format!("{output}/reference.csv"),b).unwrap();
}
