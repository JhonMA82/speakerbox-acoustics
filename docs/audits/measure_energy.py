#!/usr/bin/env python3
"""Balance RMS reproducible con los fixtures existentes. No modifica oráculos.
Uso: python docs/audits/measure_energy.py [ruta-repo]
Reutiliza únicamente la construcción de fixtures del script de primera pasada.
"""
import pathlib,subprocess,sys
prefix=pathlib.Path(__file__).with_name('measure.py').read_text().split("code=r'''")[0]
ns={};exec(compile(prefix,'measure.py fixtures','exec'),ns)
repo=ns['repo'];modules=ns['modules']
code=r'''
use speakerbox_acoustics::{Design,LegacyRecord,Calibration,OraclePoint};
fn data(n:&str)->String{std::fs::read_to_string(format!("tests/data/{n}")).unwrap()}
fn run(name:&str,d:Design,file:&str,bp:bool){
 let freqs=if bp{bp::read(&format!("tests/data/{file}_spl.txt")).into_iter().filter(|(f,_)|*f<=1000.).map(|(f,_)|f).collect::<Vec<_>>()}
 else{OraclePoint::parse_tsv(&data(file)).unwrap().into_iter().filter(|r|name.starts_with("HORN")||name.starts_with("TH")||r.freq_hz<=1000.).map(|r|r.freq_hz).collect::<Vec<_>>()};
 let mut max_abs=0.0_f64;let mut max_rel=0.0_f64;let mut max_a=0.0_f64;let mut max_equivalence=0.0_f64;let mut failures=0;let mut quality=String::new();let mut p50=None;
 for f in &freqs {
  match d.try_energy_at(*f){Err(e)=>{eprintln!("{name} {f}: {e}");failures+=1;},Ok(r)=>{
   max_abs=max_abs.max(r.balance_error_w().abs());if r.source_active_w>0.{max_rel=max_rel.max(r.balance_error_w().abs()/r.source_active_w);}
   max_a=max_a.max(r.acoustic_balance_error_w().abs());
   for t in &r.terminations{max_equivalence=max_equivalence.max((t.absorbed_power_w()-t.volume_velocity_m3_s.norm_sqr()*t.impedance_pa_s_m3.re).abs());}
   quality=format!("{:?}",r.radiated_power.quality);
  }}
 }
 if let Ok(r)=d.try_energy_at(50.) {p50=Some(r);}
 let r=p50.unwrap();
 println!("{name},{},{failures},{max_abs:.12e},{max_rel:.12e},{max_a:.12e},{max_equivalence:.12e},{quality},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",freqs.len(),r.source_active_w,r.source_resistor_loss_w,r.driver_terminal_active_w,r.coil_loss_w,r.suspension_loss_w,r.acoustic_input_w,r.acoustic_losses.total_w(),r.termination_absorbed_w(),r.radiated_power.watts.unwrap_or(f64::NAN),r.efficiency_vs_source_pct().unwrap_or(f64::NAN),r.efficiency_vs_driver_terminals_pct().unwrap_or(f64::NAN));
 assert_eq!(failures,0,"fallos de informe energético");
}
fn main(){
 println!("name,n,failures,max_balance_error_w,max_relative_source_error,max_acoustic_balance_error_w,max_flux_identity_error_w,radiation_quality,source_50_w,rg_loss_50_w,driver_terminals_50_w,coil_loss_50_w,suspension_loss_50_w,acoustic_input_50_w,acoustic_losses_50_w,termination_power_50_w,radiated_power_50_w,eta_source_50_pct,eta_terminals_50_pct");
 for (n,d,f) in od_production::cases().into_iter().chain(th_production::cases()).chain(system_production::cases()){run(n,d,f,false);}
 for (n,d,f) in bp::cases(){run(n,d,f,true);}
 for (n,rec,o) in [("HORN-1","default_record.bin","default_oracle_full.txt"),("HORN-2","design2_record.bin","design2_oracle.txt"),("HORN-3","design3_record.bin","design3_oracle.txt")] {
 let r=LegacyRecord::parse(&std::fs::read(format!("tests/data/{rec}")).unwrap()).unwrap();run(n,r.to_system_with(&Calibration::validated_60_70()).into(),o,false);}
}
'''
path=repo/'examples'/'energy_snapshot.rs'
if path.exists():raise SystemExit('El ejemplo temporal ya existe; no se sobrescribe.')
try:
 path.write_text('\n'.join(modules)+'\n'+code)
 sys.exit(subprocess.run(['cargo','run','--release','--example','energy_snapshot'],cwd=repo).returncode)
finally:path.unlink()
