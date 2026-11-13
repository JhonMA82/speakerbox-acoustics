use speakerbox_acoustics::optimization::*;
use speakerbox_acoustics::*;
fn cfg(a:Algorithm)->SearchConfig { SearchConfig {algorithm:a,max_evaluations:5000,max_iterations:2000,tolerance:1e-7,seed:42,population:24} }
#[test]
fn known_minima_both_algorithms_and_reproducibility(){
 for a in [Algorithm::NelderMead,Algorithm::DifferentialEvolution] {
  let b=[Bound{min:-5.,max:5.,locked:None};2];
  let run=||minimize(&b,&[3.,-2.],cfg(a),|x|Some((x[0]-1.).powi(2)+(x[1]+0.5).powi(2)),||false).unwrap();
  let r=run();let s=run();assert!(r.best.as_ref().unwrap().cost<1e-10);
  assert_eq!(r.best.unwrap().parameters,s.best.unwrap().parameters);
 }
 let b=[Bound{min:-3.,max:3.,locked:None};2];
 let r=minimize(&b,&[-1.,1.],cfg(Algorithm::NelderMead),|x|Some((1.-x[0]).powi(2)+100.*(x[1]-x[0]*x[0]).powi(2)),||false).unwrap();
 assert!(r.best.unwrap().cost<1e-8);
 let r=minimize(&b,&[2.,2.],cfg(Algorithm::DifferentialEvolution),|x|Some(20.+x.iter().map(|v|v*v-10.*(std::f64::consts::TAU*v).cos()).sum::<f64>()),||false).unwrap();
 assert!(r.best.unwrap().cost<1e-6);
}
#[test]
fn bounds_locked_invalid_candidates_budget_and_cancel(){
 let b=[Bound{min:0.,max:1.,locked:None},Bound{min:0.,max:2.,locked:Some(1.)}];
 let mut n=0;let mut c=cfg(Algorithm::DifferentialEvolution);c.max_evaluations=100;
 let r=minimize(&b,&[0.8,0.],c,|x|{n+=1;assert!((0. ..=1.).contains(&x[0]));assert_eq!(x[1],1.);if x[0]>0.4{None}else{Some((x[0]-0.2).powi(2))}},||false).unwrap();
 assert!(r.best.unwrap().cost<0.01);assert_eq!(n,r.evaluations);assert!(n<=100);
 let mut ticks=0;let r=minimize(&b,&[0.8,0.],c,|x|Some(x[0]),||{ticks+=1;ticks>10}).unwrap();
 assert_eq!(r.stop,Stop::Cancelled);assert!(r.best.is_some());
 let r=minimize(&b,&[0.8,0.],c,|_|None,||false).unwrap();assert_eq!(r.stop,Stop::NoValidCandidate);
}
#[test]
fn acoustic_optimization_fits_a_valid_sealed_volume(){
 let d=Driver::from_ui_units(350.,4e-4,20.,6.,18.,4.,1.,1);
 let make=|v|{let mut s=System::new(Air::default(),d);s.rear_chamber=Some(Chamber::new(v));Design::from(s)};
 let target=make(0.025).try_response_at(50.).unwrap().spl_db;
 let goals=[AcousticGoal{frequency_hz:50.,metric:AcousticMetric::SplDb,target,weight:1.,minimum:true,hard:false},AcousticGoal{frequency_hz:50.,metric:AcousticMetric::SplDb,target,weight:1.,minimum:false,hard:false}];
 let original=0.01;
 let r=minimize(&[Bound{min:0.005,max:0.05,locked:None}],&[original],cfg(Algorithm::NelderMead),|x|acoustic_cost(&make(x[0]),&goals).ok(),||false).unwrap();
 assert!(r.best.as_ref().unwrap().cost<1e-8);assert!((r.best.unwrap().parameters[0]-0.025).abs()<1e-5);assert_eq!(original,0.01);
}
