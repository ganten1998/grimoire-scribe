#!/usr/bin/env python3
"""Check the Kindle radio helper against a synthetic LIPC backend, never a real radio."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT=Path(__file__).resolve().parents[1]/'kindle/app/wifi.sh'
FAKE=r'''#!/usr/bin/env python3
import json,sys
from pathlib import Path
root=Path(__file__).parent
state=json.loads((root/'state').read_text())
a=sys.argv[1:]
if Path(sys.argv[0]).name=='lipc-get-prop':
    key=a[-1]
    if state.get('unavailable'):sys.exit(1)
    if key=='cmState': print('CONNECTED' if state['wirelessEnable'] and state['enable'] else 'IDLE')
    else:print('invalid' if state.get('invalid') else state[key])
else:
    key,value=a[-2],int(a[-1])
    with (root/'writes').open('a') as f:f.write(key+'='+str(value)+'\n')
    if state.get('fail')==key:sys.exit(1)
    if not state.get('ignore'):state[key]=value
    (root/'state').write_text(json.dumps(state))
'''
class WifiHelper(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(prefix='wifi helper ');self.root=Path(self.tmp.name)
        for name in ('lipc-get-prop','lipc-set-prop'):
            p=self.root/name;p.write_text(FAKE);p.chmod(0o755)
        self.env=dict(os.environ,PATH=str(self.root)+os.pathsep+os.environ['PATH'])
        self.state(wirelessEnable=0,enable=0)
        (self.root/'profiles').write_text('saved networks must not change')
    def tearDown(self):
        self.assertEqual((self.root/'profiles').read_text(),'saved networks must not change')
        self.tmp.cleanup()
    def state(self,**values):(self.root/'state').write_text(json.dumps(values))
    def run_helper(self,verb):return subprocess.run(['sh',str(SCRIPT),verb],env=self.env,text=True,capture_output=True,timeout=5)
    def writes(self):return (self.root/'writes').read_text().splitlines() if (self.root/'writes').exists() else []
    def test_status_is_read_only_and_does_not_claim_a_connection(self):
        r=self.run_helper('status');self.assertEqual(r.returncode,0);self.assertEqual(r.stdout,'wireless=0\nenabled=0\nconnected=0\n');self.assertEqual(self.writes(),[])
    def test_enable_order_and_idempotence(self):
        r=self.run_helper('on');self.assertEqual(r.returncode,0,r.stderr);self.assertIn('connected=1',r.stdout)
        self.assertEqual(self.writes(),['wirelessEnable=1','enable=1'])
        self.assertEqual(self.run_helper('on').returncode,0);self.assertEqual(len(self.writes()),2)
    def test_disable_order_and_idempotence(self):
        self.state(wirelessEnable=1,enable=1)
        self.assertEqual(self.run_helper('off').returncode,0)
        self.assertEqual(self.writes(),['enable=0','wirelessEnable=0'])
        self.assertEqual(self.run_helper('off').returncode,0);self.assertEqual(len(self.writes()),2)
    def test_unknown_or_unavailable_backend_is_never_written(self):
        for flags in ({'unavailable':True},{'invalid':True}):
            self.state(wirelessEnable=0,enable=0,**flags)
            self.assertNotEqual(self.run_helper('on').returncode,0);self.assertEqual(self.writes(),[])
    def test_partial_failure_is_reported_without_an_unrequested_rollback(self):
        self.state(wirelessEnable=0,enable=0,fail='enable')
        self.assertNotEqual(self.run_helper('on').returncode,0)
        self.assertEqual(self.writes(),['wirelessEnable=1','enable=1'])
        self.assertEqual(json.loads((self.root/'state').read_text())['wirelessEnable'],1)
    def test_ignored_write_does_not_claim_success(self):
        self.state(wirelessEnable=0,enable=0,ignore=True)
        self.assertNotEqual(self.run_helper('on').returncode,0)
    def test_unknown_action_is_not_executed(self):
        for verb in ('','toggle','on; touch injected'):
            self.assertEqual(self.run_helper(verb).returncode,2)
        self.assertEqual(self.writes(),[]);self.assertFalse((self.root/'injected').exists())
if __name__=='__main__':unittest.main()
