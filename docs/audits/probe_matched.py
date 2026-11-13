#!/usr/bin/env python3
"""Prueba numérica de boca adaptada compatible con ambas versiones."""
import pathlib,subprocess,sys
repo=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else '.').resolve()
code=r'''
use speakerbox_acoustics::{Air,Driver,Flare,Horn,Segment,System};
use num_complex::Complex64 as C;
fn main(){
 let air=Air::default();let area=0.02;let driver=Driver::from_ui_units(350.,4e-4,20.,6.,18.,4.,1.,1);
 let seg=Segment::new(Flare::Cylindrical,0.7,area,area);let mut h=Horn::new(vec![seg.clone()]);h.matched_mouth=true;
 let mut s=System::new(air,driver);s.horn=Some(h);
 println!("frequency_hz,reported_acoustic_w,actual_terminal_flux_w,u_squared_re_z_w,flux_equivalence_abs_w");
 for f in [10.,20.,50.,100.,500.] {
  let (zf,zr)=s.loads(f);let ud=driver.volume_velocity(f,s.eg,s.rg,zf+zr);let (p,u)=seg.matrix(&air,f).apply(zf*ud,ud);
  let flux=(p*u.conj()).re;let exact=u.norm_sqr()*(air.rho*air.c/area);
  let reported=match s.try_response_at(f){Ok(p)=>format!("{:.12e}",p.acoustical_power_w),Err(_)=>"not_available_without_observation_model".into()};
  println!("{f},{reported},{flux:.12e},{exact:.12e},{:.12e}",(flux-exact).abs());
  assert!((p-C::new(air.rho*air.c/area,0.)*u).norm()<1e-10*(1.+p.norm()));
 }
}
'''
path=repo/'examples'/'matched_probe.rs'
if path.exists():raise SystemExit('No se sobrescribe un archivo existente.')
try:
 path.write_text(code)
 sys.exit(subprocess.run(['cargo','run','--release','--example','matched_probe'],cwd=repo).returncode)
finally:path.unlink()
