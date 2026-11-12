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
name=Path(sys.argv[0]).name
if name=='lipc-get-prop':
    key=a[-1]
    if state.get('unavailable'):sys.exit(1)
    if key=='cmState': print('CONNECTED' if state.get('stale_cm') or (state['wirelessEnable'] and state['enable']) else 'IDLE')
    else:print('invalid' if state.get('invalid') else state[key])
elif name=='lipc-hash-prop':
    # wifid's hasharray properties, in the format the state names (the real one is a device probe)
    (root/'hash-stdin').write_text(sys.stdin.read())
    if state.get('hash_fail'):sys.exit(1)
    pub,prop=a[-2],a[-1]
    if pub!='com.lab126.wifid' or prop not in ('profileData','currentEssid'):sys.exit(1)
    rows=state.get('profiles',[]) if prop=='profileData' else [state.get('current',{'essid':'','netid':-1})]
    fmt=state.get('format','kv')
    q=lambda v:'"'+str(v).replace('\\','\\\\').replace('"','\\"')+'"'
    for i,r in enumerate(rows):
        if fmt=='json':print(json.dumps(r,ensure_ascii=False))
        elif fmt=='unquoted':print('{ '+', '.join(f'{k} = {v}' for k,v in r.items())+' }')
        elif fmt=='kindle':
            # the Scribe's own layout (firmware 5.20.1, read with every value masked on 2026-11-11)
            print(f'Hash Index: {i}')
            for k,v in r.items():print(f'\t{k} = ({v})' if isinstance(v,int) else f'\t{k} = {q(v)}')
            print()
        else:print(f'[{i}] = {{ '+', '.join(f'{k} = {v if isinstance(v,int) else q(v)}' for k,v in r.items())+' }')
else:
    key,value=a[-2],a[-1]
    with (root/'writes').open('a') as f:f.write(key+'='+value+'\n')
    if state.get('fail')==key:sys.exit(1)
    if a[0]=='-i' and not state.get('ignore'):state[key]=int(value)
    (root/'state').write_text(json.dumps(state))
'''
class WifiHelper(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(prefix='wifi helper ');self.root=Path(self.tmp.name)
        for name in ('lipc-get-prop','lipc-set-prop','lipc-hash-prop'):
            p=self.root/name;p.write_text(FAKE);p.chmod(0o755)
        self.env=dict(os.environ,PATH=str(self.root)+os.pathsep+os.environ['PATH'])
        self.state(wirelessEnable=0,enable=0)
        (self.root/'profiles').write_text('saved networks must not change')
    def tearDown(self):
        self.assertEqual((self.root/'profiles').read_text(),'saved networks must not change')
        self.tmp.cleanup()
    def state(self,**values):(self.root/'state').write_text(json.dumps(values))
    def run_helper(self,verb,*args):return subprocess.run(['sh',str(SCRIPT),verb,*args],env=self.env,text=True,capture_output=True,timeout=5)
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
    def test_stale_connected_state_cannot_override_disabled_radio_flags(self):
        self.state(wirelessEnable=1,enable=1,stale_cm=True)
        r=self.run_helper('off');self.assertEqual(r.returncode,0)
        self.assertEqual(r.stdout,'wireless=0\nenabled=0\nconnected=0\n')
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
        for verb in ('','toggle','on; touch injected','join'):
            self.assertEqual(self.run_helper(verb).returncode,2)
        self.assertEqual(self.writes(),[]);self.assertFalse((self.root/'injected').exists())
    # ---- the saved networks (2.1.0-beta.2): read-only verbs, and a switch to a saved one ----
    PROFILES=[{'essid':'Home','secured':'yes','psk':'PSK-one essid="leak"','netid':1,'store_nw_user_pref':0},
              {'essid':'Caf\u00e9 \u732b','psk':'PSK-two','netid':2},
              {'essid':'Open','secured':'no','netid':3}]
    def saved(self,out):return [bytes.fromhex(l[6:]).decode() for l in out.splitlines() if l.startswith('saved=')]
    def no_password(self,r):
        for psk in ('PSK-one','PSK-two','leak'):
            for text in (r.stdout,r.stderr):
                self.assertNotIn(psk,text);self.assertNotIn(psk.encode().hex(),text)
    def test_networks_lists_the_saved_names_in_hex_and_never_a_password(self):
        for fmt in ('kv','json','kindle'):
            self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,current={'essid':'Home','netid':1},format=fmt)
            r=self.run_helper('networks');self.assertEqual(r.returncode,0,r.stderr)
            self.assertEqual(self.saved(r.stdout),['Home','Caf\u00e9 \u732b','Open'],fmt)
            lines=r.stdout.splitlines()
            self.assertEqual(lines[:4],['wireless=1','enabled=1','connected=1','current='+'Home'.encode().hex()])
            self.assertEqual(lines[-1],'list=1');self.no_password(r)
            # read as KOReader reads them: an empty hasharray in
            self.assertEqual((self.root/'hash-stdin').read_text().strip(),'{}')
        self.assertEqual(self.writes(),[])
    def test_the_network_connected_to_is_named_only_while_connected(self):
        self.state(wirelessEnable=0,enable=0,profiles=self.PROFILES,current={'essid':'Home','netid':1})
        r=self.run_helper('current');self.assertEqual(r.returncode,0)
        self.assertEqual(r.stdout,'wireless=0\nenabled=0\nconnected=0\ncurrent=\n')
        self.state(wirelessEnable=1,enable=1,current={'essid':'','netid':-1})
        self.assertEqual(self.run_helper('current').stdout.splitlines()[-1],'current=')
        self.state(wirelessEnable=1,enable=1,current={'essid':'Open','netid':3})
        self.assertEqual(self.run_helper('current').stdout.splitlines()[-1],'current='+'Open'.encode().hex())
        self.assertEqual(self.writes(),[])
    def test_a_format_it_does_not_know_lists_no_name_at_all(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,format='unquoted')
        r=self.run_helper('networks');self.assertEqual(r.returncode,0)
        self.assertEqual(self.saved(r.stdout),[]);self.assertEqual(r.stdout.splitlines()[-1],'list=0');self.no_password(r)
        self.assertIn('current=',r.stdout)
    def test_a_failed_read_lists_nothing_but_still_reports_the_radio(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,hash_fail=True)
        r=self.run_helper('networks');self.assertEqual(r.returncode,0)
        self.assertEqual(r.stdout,'wireless=1\nenabled=1\nconnected=1\ncurrent=\nlist=0\n')
        (self.root/'lipc-hash-prop').unlink()
        self.assertEqual(self.run_helper('networks').stdout.splitlines()[-1],'list=0')
        self.state(wirelessEnable=1,enable=1,unavailable=True)
        self.assertNotEqual(self.run_helper('networks').returncode,0)
    def test_names_that_could_not_be_given_back_are_left_out(self):
        rows=[{'essid':'Bob\'s "net"'},{'essid':'back\\slash'},{'essid':'tab\there'},{'essid':'x'*33},
              {'essid':''},{'essid':'Fine'},{'essid':'Fine'},{'essid':'y'*32}]
        self.state(wirelessEnable=1,enable=1,profiles=rows)
        r=self.run_helper('networks')
        self.assertEqual(self.saved(r.stdout),['Fine','y'*32]);self.assertEqual(r.stdout.splitlines()[-1],'list=1')
    def test_join_asks_wifid_for_a_saved_network_and_nothing_else(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES)
        r=self.run_helper('join','Home'.encode().hex());self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(self.writes(),['ensureConnection=wifi:Home']);self.assertTrue(r.stdout.startswith('wireless=1\n'))
        self.no_password(r)
        # with the radio off, it comes on first, in the order Turn on uses
        self.state(wirelessEnable=0,enable=0,profiles=self.PROFILES);(self.root/'writes').unlink()
        self.assertEqual(self.run_helper('join','Caf\u00e9 \u732b'.encode().hex()).returncode,0)
        self.assertEqual(self.writes(),['wirelessEnable=1','enable=1','ensureConnection=wifi:Caf\u00e9 \u732b'])
    def test_join_refuses_anything_but_a_saved_network_before_any_change(self):
        self.state(wirelessEnable=0,enable=0,profiles=self.PROFILES)
        self.assertEqual(self.run_helper('join','Nope'.encode().hex()).returncode,3)
        for bad in ('','486F6D65','486f6d6','zz','48 6f','486f6d65; touch injected','41'*33,'-1'):
            self.assertEqual(self.run_helper('join',bad).returncode,2,bad)
        self.state(wirelessEnable=0,enable=0,profiles=self.PROFILES,hash_fail=True)
        self.assertEqual(self.run_helper('join','Home'.encode().hex()).returncode,3)
        self.state(wirelessEnable=0,enable=0,profiles=self.PROFILES,format='unquoted')
        self.assertEqual(self.run_helper('join','Home'.encode().hex()).returncode,3)
        self.state(wirelessEnable=0,enable=0,profiles=self.PROFILES,unavailable=True)
        self.assertNotEqual(self.run_helper('join','Home'.encode().hex()).returncode,0)
        self.assertEqual(self.writes(),[]);self.assertFalse((self.root/'injected').exists())
    def test_a_hostile_name_is_passed_as_data(self):
        name="$(touch injected) `touch i2`;x"
        self.state(wirelessEnable=1,enable=1,profiles=[{'essid':name,'psk':'PSK-one'}])
        self.assertEqual(self.saved(self.run_helper('networks').stdout),[name])
        self.assertEqual(self.run_helper('join',name.encode().hex()).returncode,0)
        self.assertEqual(self.writes(),['ensureConnection=wifi:'+name])
        self.assertFalse((self.root/'injected').exists());self.assertFalse((self.root/'i2').exists())
    def test_a_failed_switch_is_reported(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,fail='ensureConnection')
        self.assertNotEqual(self.run_helper('join','Home'.encode().hex()).returncode,0)
if __name__=='__main__':unittest.main()
