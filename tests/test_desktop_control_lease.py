import os,socket,tempfile,unittest
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"tools"))
from desktop_control_lease import reserve,LeaseConflict,prune_record,local_request_authorized
class DesktopLeaseTests(unittest.TestCase):
 def test_second_daemon_cannot_claim_same_uds(self):
  with tempfile.TemporaryDirectory() as root:
   path=Path(root)/"runtime.sock"
   first=reserve(path)
   try:
    with self.assertRaises(LeaseConflict):reserve(path)
    self.assertEqual(path.stat().st_mode&0o777,0o600)
   finally:
    first.close();path.unlink()
 def test_two_daemons_can_bind_separate_ephemeral_socket_paths(self):
  with tempfile.TemporaryDirectory() as root:
   a=reserve(Path(root)/"a.sock");b=reserve(Path(root)/"b.sock")
   self.assertNotEqual(a.getsockname(),b.getsockname())
   a.close();b.close()
 def test_os_peer_identity_does_not_replace_shared_auth(self):
  self.assertFalse(local_request_authorized(True,False,"giw:admin"))
  self.assertFalse(local_request_authorized(False,True,"giw:jobs"))
  self.assertTrue(local_request_authorized(True,True,"giw:read"))
 def test_six_hour_window_and_severity(self):
  now=25_000_000
  self.assertTrue(prune_record(now-6*3600*1000,now,"ERROR"))
  self.assertFalse(prune_record(now-6*3600*1000-1,now,"WARN"))
  with self.assertRaises(ValueError):prune_record(now,now,"SECRET")
if __name__=="__main__":unittest.main()
