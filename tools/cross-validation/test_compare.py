import unittest
from compare import compare
class Tests(unittest.TestCase):
 def test_known_errors_bands_and_phase(self):
  a=[dict(frequency_hz=f,spl_db=s,phase_deg=p) for f,s,p in [(10,2,179),(50,4,-179),(200,6,10)]]
  b=[dict(frequency_hz=f,spl_db=s,phase_deg=p) for f,s,p in [(10,1,-179),(50,2,179),(200,3,10)]]
  r=compare(a,b,{'spl_db':1.5})['metrics'];self.assertAlmostEqual(r['spl_db']['rms'],(14/3)**.5);self.assertEqual(r['spl_db']['bias'],2);self.assertEqual(r['spl_db']['out_of_tolerance'],2);self.assertEqual(r['phase_deg']['max_abs'],2);self.assertNotIn('power_w',r)
 def test_mismatch_rejected(self):
  for a,b,tol in [([dict(frequency_hz=1)],[dict(frequency_hz=2)],None),([],[],None),([dict(frequency_hz=1)],[dict(frequency_hz=1)],None),([dict(frequency_hz=1,spl_db=1)],[dict(frequency_hz=1,spl_db=1)],{'spl_db':float('nan')})]:
   with self.assertRaises(ValueError):compare(a,b,tol)
if __name__=='__main__':unittest.main()
