//! Síntesis de alineamientos ideales y comprobación mediante el solver completo.
use crate::{Air,Driver,System,Chamber,Port,EndCorrection,SolveError,Design};
use num_complex::Complex64 as C;
use std::f64::consts::{PI,TAU};
#[derive(Clone,Copy,Debug)]
pub enum SealedFamily { Butterworth, Bessel, Chebyshev {ripple_db:f64} }
pub fn sealed(driver:&Driver,air:&Air,family:SealedFamily)->Result<crate::reference_design::SealedAlignment,SolveError>{
 driver.validate()?;air.validate()?;
 let q=match family {
  SealedFamily::Butterworth=>1./2_f64.sqrt(),SealedFamily::Bessel=>1./3_f64.sqrt(),
  SealedFamily::Chebyshev{ripple_db}=>{
   if !ripple_db.is_finite()||ripple_db<=0. {return Err(SolveError::InvalidParameter("ripple dB"));}
   // Resonant peak squared = Q^4/(Q²-1/4), high-frequency gain one.
   let peak=10_f64.powf(ripple_db/10.);((peak+(peak*(peak-1.)).sqrt())/2.).sqrt()
  }
 };
 crate::reference_design::sealed_alignment(driver.fs(air),driver.qts(air),driver.vas(air),q)
}
#[derive(Clone,Copy,Debug)]
pub enum VentedFamily {ButterworthB4,Bessel4,Chebyshev4{ripple_db:f64},SubChebyshev4,QuasiButterworth3}
#[derive(Clone,Copy,Debug)]
pub struct VentedAlignment {pub volume_m3:f64,pub tuning_hz:f64,pub fs_hz:f64,pub qts:f64,pub alpha:f64,pub f3_hz:f64}
impl VentedAlignment {
 /// Ideal lossless lumped-port high-pass (normalized to high-frequency gain).
 pub fn transfer(&self,f:f64)->C {
  let s=C::new(0.,f/self.fs_hz);let h=self.tuning_hz/self.fs_hz;
  s.powu(4)/(s.powu(4)+s.powu(3)/self.qts+s*s*(1.+self.alpha+h*h)+s*h*h/self.qts+C::new(h*h,0.))
 }
 /// Geometría física sin recortar longitud negativa. Área y end correction
 /// son elecciones del diseñador, no constantes universales del alineamiento.
 pub fn to_system(&self,driver:Driver,air:Air,area:f64,end:EndCorrection)->Result<System,SolveError>{
  crate::error::require_positive(area,"alignment port area")?;
  let length=air.c*air.c*area/(self.volume_m3*(TAU*self.tuning_hz).powi(2))-end.delta(area);
  if !length.is_finite()||length<=0. {return Err(SolveError::InvalidParameter("alignment port too short for end correction"));}
  let mut sys=System::new(air,driver);sys.rear_chamber=Some(Chamber::new(self.volume_m3));sys.port=Some(Port::new(area,length,end));sys.validate()?;Ok(sys)
 }
 /// Desviación frente a filtro ideal, no una afirmación de igualdad exacta.
 pub fn solver_deviation_db(&self,driver:Driver,air:Air,area:f64,frequencies:&[f64])->Result<Vec<f64>,SolveError>{
  let sys=self.to_system(driver,air,area,EndCorrection::None)?;
  let flat=air.rho*driver.sd*driver.bl*sys.eg/(TAU*driver.re*driver.mms(&air));
  frequencies.iter().map(|&f|{crate::error::check_frequency(f)?;let expected=20.*(flat*self.transfer(f).norm()/air.p_ref).log10();Ok(sys.try_response_at(f)?.spl_db-expected)}).collect()
 }
}
fn finish(driver:&Driver,air:&Air,alpha:f64,h:f64)->Result<VentedAlignment,SolveError>{
 if !alpha.is_finite()||alpha<=0.||!h.is_finite()||h<=0. {return Err(SolveError::InvalidParameter("alignment not realizable"));}
 let mut a=VentedAlignment{volume_m3:driver.vas(air)/alpha,tuning_hz:driver.fs(air)*h,fs_hz:driver.fs(air),qts:driver.qts(air),alpha,f3_hz:0.};
 let threshold=1./2_f64.sqrt();let mut lo=a.fs_hz*1e-5;let mut hi=lo;
 for _ in 0..10000 {hi*=1.01;if a.transfer(hi).norm()>=threshold {break;}lo=hi;}
 if a.transfer(hi).norm()<threshold{return Err(SolveError::InvalidResponse("alignment cutoff not bracketed"));}
 for _ in 0..60 {let mid=(lo+hi)/2.;if a.transfer(mid).norm()<threshold{lo=mid;}else{hi=mid;}}
 a.f3_hz=(lo+hi)/2.;Ok(a)
}
// From low-pass conjugate poles -k sin(theta)+j cos(theta). Independent
// polynomial multiplication, then low-pass -> high-pass coefficient reversal.
fn cheb_prototype(k:f64)->[f64;4]{
 let mut p=vec![C::new(1.,0.)];
 for i in 0..4 {let theta=PI*(2*i+1) as f64/8.;let root=C::new(-k*theta.sin(),theta.cos());let mut next=vec![C::new(0.,0.);p.len()+1];for (j,&v) in p.iter().enumerate(){next[j]+=v;next[j+1]-=v*root;}p=next;}
 [p[3].re/p[4].re,p[2].re/p[4].re,p[1].re/p[4].re,1./p[4].re]
}
fn required_q(b:[f64;4])->f64{(b[3]/(b[0]*b[2])).sqrt()}
/// Synthesis without loss corrections: exact only in the lumped linear T/S
/// model (Le=0, constant Mms, no port/wall/radiation resistance). B4/Bessel/C4
/// require compatible Qts: the driver is never silently changed to match.
pub fn vented(driver:&Driver,air:&Air,family:VentedFamily)->Result<VentedAlignment,SolveError>{
 driver.validate()?;air.validate()?;let q=driver.qts(air);
 if matches!(family,VentedFamily::QuasiButterworth3){
  let h2=1./(8.*q*q*(1.-q*q));let a2=1./(2.*q*q);
  // |D(jw)|²=w8+B²w2+h4 (both w6 and w4 terms vanish).
  if q> (PI/8.).sin(){return Err(SolveError::InvalidParameter("QB3 requires Qts <= sin(pi/8)"));}
  return finish(driver,air,a2-1.-h2,h2.sqrt());
 }
 let b=match family {
  VentedFamily::ButterworthB4=>cheb_prototype(1.),
  VentedFamily::Bessel4=>[1.,3./7.,2./21.,1./105.],
  VentedFamily::Chebyshev4{ripple_db}=>{
   if !ripple_db.is_finite()||ripple_db<=0. {return Err(SolveError::InvalidParameter("ripple dB"));}
   let epsilon=(10_f64.powf(ripple_db/10.)-1.).sqrt();cheb_prototype(((1./epsilon).asinh()/4.).tanh())
  },
  VentedFamily::SubChebyshev4=>{
   let mut lo=1.;let mut hi=2.;while required_q(cheb_prototype(hi))>q&&hi<1e5{hi*=2.;}
   if q>required_q(cheb_prototype(lo))||q<required_q(cheb_prototype(hi)){return Err(SolveError::InvalidParameter("SC4 outside realizable conjugate-pole domain"));}
   for _ in 0..80{let m=(lo+hi)/2.;if required_q(cheb_prototype(m))>q{lo=m;}else{hi=m;}}
   cheb_prototype((lo+hi)/2.)
  },VentedFamily::QuasiButterworth3=>unreachable!()
 };
 if (required_q(b)-q).abs()>1e-7{return Err(SolveError::InvalidParameter("driver Qts incompatible with exact prototype"));}
 let r2=b[2]/(b[0]*b[3]);let h2=b[3]*r2*r2;finish(driver,air,b[1]*r2-1.-h2,h2.sqrt())
}
/// EBS is not unique. Numerically fit a specified shelf depth/corner using the
/// full solver, explicit physical bounds and a user-selected port area.
#[allow(clippy::too_many_arguments)]
pub fn extended_bass_shelf(driver:Driver,air:Air,volume_bounds:(f64,f64),tuning_bounds:(f64,f64),port_area:f64,corner_hz:f64,shelf_db:f64,budget:usize)->Result<(System,crate::optimization::SearchResult),SolveError>{
 use crate::optimization::*;
 driver.validate()?;air.validate()?;
 if !corner_hz.is_finite()||corner_hz<=0.||!shelf_db.is_finite()||shelf_db>=0.||volume_bounds.0<=0.||tuning_bounds.0<=0.{return Err(SolveError::InvalidParameter("EBS specification"));}
 let bounds=[Bound{min:volume_bounds.0,max:volume_bounds.1,locked:None},Bound{min:tuning_bounds.0,max:tuning_bounds.1,locked:None}];
 let make=|x:&[f64]|{let a=VentedAlignment{volume_m3:x[0],tuning_hz:x[1],fs_hz:driver.fs(&air),qts:driver.qts(&air),alpha:driver.vas(&air)/x[0],f3_hz:0.};a.to_system(driver,air,port_area,EndCorrection::None)};
 let fs=driver.fs(&air);let flat=air.rho*driver.sd*driver.bl*2.83/(TAU*driver.re*driver.mms(&air));
 let f:Vec<_>=(0..32).map(|i|corner_hz*0.5*(fs*8./(corner_hz*0.5)).powf(i as f64/31.)).collect();
 let gain=10_f64.powf(shelf_db/20.);
 let result=minimize(&bounds,&[(volume_bounds.0+volume_bounds.1)/2.,(tuning_bounds.0+tuning_bounds.1)/2.],SearchConfig{algorithm:Algorithm::NelderMead,max_evaluations:budget,max_iterations:budget,tolerance:1e-6,seed:1,population:16},|x|{
  let d=Design::from(make(x).ok()?);let mut sum=0.;
  for &freq in &f {let hp=1./(1.+(corner_hz/freq).powi(8)).sqrt();let shelf=(freq*freq+(gain*fs*2.).powi(2)).sqrt()/(freq*freq+(fs*2.).powi(2)).sqrt();let target=20.*(flat*hp*shelf/air.p_ref).log10();sum+=(d.try_response_at(freq).ok()?.spl_db-target).powi(2);}
  Some(sum/f.len() as f64)
 },||false)?;
 let sys=make(&result.best.as_ref().ok_or(SolveError::InvalidResponse("no valid EBS proposal"))?.parameters)?;
 Ok((sys,result))
}
