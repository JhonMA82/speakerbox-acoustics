use speakerbox_acoustics::{Air,Driver,Flare,Horn,Segment,System};
#[test]
fn extreme_attenuation_must_not_return_an_apparently_valid_response(){
 let d=Driver::from_ui_units(350.,4e-4,20.,6.,18.,4.,1.,1);
 let mut s=System::new(Air::default(),d);s.horn=Some(Horn::new(vec![Segment::new(Flare::Cylindrical,0.8,0.02,0.02).with_flow_resistivity(1e8)]));
 assert!(s.try_response_at(10.).is_err());
}
