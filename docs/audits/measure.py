#!/usr/bin/env python3
"""Reutiliza fixtures de producción, sin modificar datos ni calibración.
Uso: python docs/audits/measure.py [ruta-repo]
Genera un ejemplo temporal, imprime métricas CSV y lo elimina al terminar.
Fase acústica: comparación diagnóstica contra columna UPhase, no tolerancia.
"""
import pathlib,sys,subprocess
repo=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else '.').resolve()
modules=[]
fixtures={
'od_production': 'vec![("OD-A",od().into(),"od_a_oracle.txt"),("OD1-B",od1().into(),"od1_b_oracle.txt"),("OD2-C",od2().into(),"od2_c_oracle.txt")]',
'th_production': 'vec![("TH1",th1().into(),"th1_oracle.txt"),("TH2",th2().into(),"th2_oracle.txt"),("TH1-S2S3",th1_s2s3().into(),"th1_s2s3_oracle.txt")]',
'system_production': 'vec![("SEALED-A",sealed().into(),"sealed_a_oracle.txt"),("VENTED-A",vented().into(),"vented_a_oracle.txt"),("PR-A",passive_radiator().into(),"pr_a_oracle.txt")]',
}
for name,body in fixtures.items():
 source=(repo/'tests'/f'{name}.rs').read_text().replace('//!','//')
 modules.append(f'#[allow(dead_code,unused_imports)] mod {name} {{\n{source}\npub fn cases()->Vec<(&\'static str,speakerbox_acoustics::Design,&\'static str)>{{{body}}}\n}}')
# BP oracles are three separate curves, without phase/power data.
source=(repo/'tests/bp_production.rs').read_text().replace('//!','//')
modules.append(f'#[allow(dead_code,unused_imports)] mod bp {{\n{source}\npub fn cases()->Vec<(&\'static str,speakerbox_acoustics::Design,&\'static str)>{{vec![("BP4-A",build("tests/data/bp4_a_record.txt").into(),"bp4_a"),("BP6P-A",build("tests/data/bp6_a_record.txt").into(),"bp6_a"),("BP6S-A",build("tests/data/bp6s_a_record.txt").into(),"bp6s_a")]}}\npub fn read(s:&str)->Vec<(f64,f64)>{{curve(s)}}\n}}')
code=r'''
use speakerbox_acoustics::{Design,LegacyRecord,Calibration,OraclePoint};
fn data(n:&str)->String {std::fs::read_to_string(format!("tests/data/{n}")).unwrap()}
fn rms(v:&[f64])->f64 {(v.iter().map(|x|x*x).sum::<f64>()/v.len() as f64).sqrt()}
fn maximum(v:&[f64])->f64 {v.iter().map(|x|x.abs()).fold(0.,f64::max)}
fn phase(a:f64,b:f64)->f64 {(a-b+180.).rem_euclid(360.)-180.}
fn measure(name:&str,d:Design,file:&str,bp:bool){
 let full=if bp {vec![]}else{data(file).lines().skip(1).filter(|l|!l.trim().is_empty()).map(|l|l.split('\t').map(|s|s.replace(',',".").parse::<f64>().unwrap_or(f64::NAN)).collect::<Vec<_>>()).filter(|r|name.starts_with("HORN")||name.starts_with("TH")||r[0]<=1000.).collect::<Vec<_>>()};
 let oracle=if bp {
  let s=bp::read(&format!("tests/data/{file}_spl.txt"));let z=bp::read(&format!("tests/data/{file}_ze.txt"));let x=bp::read(&format!("tests/data/{file}_xd.txt"));
  s.iter().zip(z).zip(x).filter(|((s,_),_)|s.0<=1000.).map(|((&(f,s),(_,z)),(_,x))|OraclePoint{freq_hz:f,spl_db:s,ze_mag:z,xd_mm:x,ze_phase_deg:0.}).collect::<Vec<_>>()
 }else{OraclePoint::parse_tsv(&data(file)).unwrap().into_iter().filter(|r|name.starts_with("HORN")||name.starts_with("TH")||r.freq_hz<=1000.).collect()};
 let freqs=oracle.iter().map(|o|o.freq_hz).collect::<Vec<_>>();let points=d.response(&freqs);
 let mut spl=vec![];let mut ze=vec![];let mut xd=vec![];let mut zp=vec![];let mut ap=vec![];let mut gd=vec![];let mut pw=vec![];let mut eff=vec![];
 for (i,(p,o)) in points.iter().zip(&oracle).enumerate(){
  p.validate().unwrap();spl.push(p.spl_db-o.spl_db);ze.push((p.ze_mag/o.ze_mag-1.)*100.);if o.xd_mm>0.02{xd.push((p.xd_mm/o.xd_mm-1.)*100.);}
  if !bp {let r=&full[i];zp.push(phase(p.ze_phase_deg,o.ze_phase_deg));ap.push(phase(p.phase_deg+90.,r[8]));gd.push(p.group_delay_ms-r[10]);let power=r[11]*r[13]/100.;if power>0.{pw.push((p.power_w/power-1.)*100.);}eff.push(p.efficiency_pct-r[11]);}
 }
 println!("{name},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9}",oracle.len(),rms(&spl),maximum(&spl),rms(&ze),maximum(&ze),rms(&xd),maximum(&xd),rms(&zp),rms(&ap),rms(&gd),rms(&pw),rms(&eff));
 // Trace de salida para medir diferencias entre commits, no un nuevo oráculo.
 if let Ok(dir)=std::env::var("AUDIT_TRACE_DIR") {std::fs::create_dir_all(&dir).unwrap();let text=points.iter().map(|p|format!("{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",p.freq_hz,p.spl_db,p.ze_mag,p.xd_mm,p.ze_phase_deg,p.phase_deg,p.group_delay_ms,p.power_w,p.efficiency_pct)).collect::<String>();std::fs::write(format!("{dir}/{name}.csv"),text).unwrap();}
}
fn main(){
 println!("name,n,spl_rms_db,spl_max_db,ze_rms_pct,ze_max_pct,xd_rms_pct,xd_max_pct,ze_phase_rms_deg,acoustic_UPhase_rms_deg,delay_rms_ms,power_rms_pct,efficiency_rms_percentage_points");
 for (n,d,f) in od_production::cases().into_iter().chain(th_production::cases()).chain(system_production::cases()){measure(n,d,f,false);}
 for (n,d,f) in bp::cases(){measure(n,d,f,true);}
 for (n,rec,o) in [("HORN-1","default_record.bin","default_oracle_full.txt"),("HORN-2","design2_record.bin","design2_oracle.txt"),("HORN-3","design3_record.bin","design3_oracle.txt")] {
 let r=LegacyRecord::parse(&std::fs::read(format!("tests/data/{rec}")).unwrap()).unwrap();measure(n,r.to_system_with(&Calibration::validated_60_70()).into(),o,false);}
}
'''
path=repo/'examples'/'audit_snapshot.rs'
if path.exists():raise SystemExit('El ejemplo temporal ya existe; no se sobrescribe.')
try:
 path.write_text('\n'.join(modules)+'\n'+code)
 sys.exit(subprocess.run(['cargo','run','--release','--example','audit_snapshot'],cwd=repo).returncode)
finally:path.unlink()
