#!/usr/bin/env python3
"""Check the Kindle radio helper against a synthetic LIPC backend, never a real radio."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT=Path(__file__).resolve().parents[1]/'kindle/app/wifi.sh'
FAKE_WPA=r'''#!/usr/bin/env python3
import json,re,sys
from pathlib import Path
root=Path(__file__).parent
state=json.loads((root/'state').read_text())
with (root/'writes').open('a') as f:f.write('wpa_cli='+sys.argv[-1]+'\n')
with (root/'wpa-argv').open('a') as f:f.write(json.dumps(sys.argv)+'\n')
if 'get_capability' in sys.argv:
    print(state.get('cap_key_mgmt','WPA-PSK WPA-EAP WPA-EAP-SHA256 SAE'));sys.exit(0)
if not any(c in sys.argv for c in ('list_networks','remove_network','save_config','disable_network','enable_network','disconnect','status','add_network')):
    # an interactive session: the commands (and any secret in them) arrive on stdin, never in argv
    cmds=sys.stdin.read().splitlines()
    with (root/'wpa-commands').open('a') as f:f.write('\n'.join(cmds)+'\n')
    # The real wpa_cli prints its prompt before each answer in an interactive session.
    for line in cmds:
        if state.get('wpa_fail_command') and line.startswith(state['wpa_fail_command']):print('> FAIL')
        elif state.get('wpa_prompt')=='cr':sys.stdout.write('>OK\r\n')
        else:print('> OK')
    sys.exit(0)
if 'add_network' in sys.argv:
    print(state.get('next_id',0));sys.exit(0)
if 'list_networks' in sys.argv:
    for p in state.get('wpa_networks',[]):print(f"{p['id']}\t{p['ssid']}\t{p['ssid']}\t[ESSID]")
    sys.exit(0)
if 'remove_network' in sys.argv:
    _id=sys.argv[-1]
    going=[p for p in state.get('wpa_networks',[]) if str(p['id'])==_id]
    state['wpa_networks']=[p for p in state.get('wpa_networks',[]) if str(p['id'])!=_id]
    for p in going:state['profiles']=[q for q in state.get('profiles',[]) if q.get('essid')!=p['ssid']]
    (root/'state').write_text(json.dumps(state));print('OK');sys.exit(0)
if 'save_config' in sys.argv:
    (root/'state').write_text(json.dumps(state));print('OK');sys.exit(0)
if state.get('wpa_fail'):sys.exit(1)
if 'disable_network' in sys.argv:
    state.setdefault('disabled',[]).append(sys.argv[-1])
    if state.get('reenable'):state['disabled']=[]
if 'enable_network' in sys.argv:
    for _id in [a for a in sys.argv[1:] if a.isdigit()]:
        if _id in state.get('disabled',[]):state['disabled'].remove(_id)
if sys.argv[-1]=='disconnect':state['linked']=False
if sys.argv[-1]=='status':print('bssid=00:11:22:33:44:55\nssid=Fake\nid=0\nwpa_state=COMPLETED')
(root/'state').write_text(json.dumps(state))
'''
FAKE=r'''#!/usr/bin/env python3
import json,re,sys
from pathlib import Path
root=Path(__file__).parent
state=json.loads((root/'state').read_text())
a=sys.argv[1:]
name=Path(sys.argv[0]).name
if name=='lipc-get-prop':
    key=a[-1]
    if state.get('unavailable'):sys.exit(1)
    if state.get('late_fail'):
        n=state.get('gets',0)+1;state['gets']=n;(root/'state').write_text(json.dumps(state))
        if n>2:sys.exit(1)
    if key=='scanState' and state.get('scan_fail'):sys.exit(1)
    if key=='scanState': print(state.get('scan_state','idle'))
    elif key=='cmState':
        if state.get('cm_fail'):sys.exit(1)
        n=state.get('cm_calls',0)+1;state['cm_calls']=n
        linked=state.get('stale_cm') or (state['wirelessEnable'] and state['enable'] and state.get('linked',True))
        if state.get('rejoin_after') and n>=state['rejoin_after'] and not state.get('disabled'):linked=True
        (root/'state').write_text(json.dumps(state))
        print('CONNECTED' if linked else 'IDLE')
    elif key=='cmProfile': print(state.get('cmProfile',''))
    else:print('invalid' if state.get('invalid') else state[key])
elif name=='lipc-hash-prop':
    # wifid's hasharray properties, in the format the state names (the real one is a device probe)
    data=sys.stdin.read();(root/'hash-stdin').write_text(data)
    if state.get('hash_fail'):sys.exit(1)
    pub,prop=a[-2],a[-1]
    if pub!='com.lab126.wifid' or prop not in ('profileData','currentEssid','scanList','createProfile','netidValue'):sys.exit(1)
    if prop=='createProfile':(root/'create-stdin').write_text(data);sys.exit(0)
    if prop=='profileData':rows=state.get('profiles',[])
    elif prop=='scanList':rows=state.get('scan_list',[])
    elif prop=='netidValue':rows=state.get('netid_values',[])
    else:rows=[state.get('current',{'essid':'','netid':-1})]
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
    if key=='scan':state['scan_state']='idle'
    if key=='deleteProfile':
        if a[0]=='-i':
            if state.get('delete_int_fail'):sys.exit(1)
            gone={p.get('essid') for p in state.get('profiles',[]) if str(p.get('netid'))==value}
            gone|={r.get('essid') for r in state.get('netid_values',[]) if str(r.get('netid'))==value}
            state['profiles']=[p for p in state.get('profiles',[]) if p.get('essid') not in gone]
        else:
            if state.get('delete_string_fail'):sys.exit(1)
            state['profiles']=[p for p in state.get('profiles',[]) if p.get('essid')!=value]
    if key=='cmDisconnect' and not state.get('cm_noop'):state['linked']=False
    if key=='ensureConnection':state['linked']=True
    if a[0]=='-i' and not state.get('ignore'):state[key]=int(value)
    (root/'state').write_text(json.dumps(state))
'''
class WifiHelper(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(prefix='wifi helper ');self.root=Path(self.tmp.name)
        for name in ('lipc-get-prop','lipc-set-prop','lipc-hash-prop'):
            p=self.root/name;p.write_text(FAKE);p.chmod(0o755)
        p=self.root/'wpa_cli';p.write_text(FAKE_WPA);p.chmod(0o755)
        # wpa_passphrase prints the passphrase in its comment line, as the real one does; the helper must
        # never let that line reach its own output. It hashes from stdin, so the passphrase is not in argv.
        p=self.root/'wpa_passphrase'
        p.write_text('#!/usr/bin/env python3\nimport sys\npw=sys.stdin.readline().rstrip("\\n")\nprint("network={")\nprint("\\tssid=\\""+sys.argv[1]+"\\"")\nprint("\\t#psk=\\""+pw+"\\"")\nprint("\\tpsk="+"ab"*32)\nprint("}")\n')
        p.chmod(0o755)
        self.env=dict(os.environ,PATH=str(self.root)+os.pathsep+os.environ['PATH'])
        self.state(wirelessEnable=0,enable=0)
        (self.root/'profiles').write_text('saved networks must not change')
    def tearDown(self):
        self.assertEqual((self.root/'profiles').read_text(),'saved networks must not change')
        self.tmp.cleanup()
    def state(self,**values):(self.root/'state').write_text(json.dumps(values))
    def run_helper(self,verb,*args,data=None):return subprocess.run(['sh',str(SCRIPT),verb,*args],env=self.env,text=True,input=data,capture_output=True,timeout=5)
    def writes(self):return (self.root/'writes').read_text().splitlines() if (self.root/'writes').exists() else []
    def test_status_is_read_only_and_does_not_claim_a_connection(self):
        r=self.run_helper('status');self.assertEqual(r.returncode,0);self.assertEqual(r.stdout,'wireless=0\nenabled=0\nconnected=0\n');self.assertEqual(self.writes(),[])
    def test_enable_order_and_idempotence(self):
        r=self.run_helper('on');self.assertEqual(r.returncode,0,r.stderr);self.assertIn('connected=1',r.stdout)
        self.assertEqual(self.writes(),['wirelessEnable=1','enable=1','cmConnMode=auto'])
        self.assertEqual(self.run_helper('on').returncode,0)
        # turning on again writes no radio flag; the manager's automatic mode is restated
        self.assertEqual(self.writes().count('wirelessEnable=1'),1)
        self.assertEqual(self.writes().count('enable=1'),1)
    def test_disable_order_and_idempotence(self):
        self.state(wirelessEnable=1,enable=1)
        self.assertEqual(self.run_helper('off').returncode,0)
        self.assertEqual(self.writes(),['enable=0','wirelessEnable=0'])
        self.assertEqual(self.run_helper('off').returncode,0);self.assertEqual(len(self.writes()),2)
    def test_disconnect_leaves_the_radio_on_and_names_no_network(self):
        self.state(wirelessEnable=1,enable=1,linked=True,profiles=self.PROFILES,cmProfile='Home',current={'essid':'Home','netid':1})
        r=self.run_helper('disconnect');self.assertEqual(r.returncode,0,r.stderr)
        # manual first, so the connection manager cannot join the profile again after the drop
        self.assertEqual(self.writes(),['cmConnMode=manual','wpa_cli=status','cmDisconnect=Home','wpa_cli=disconnect','wpa_cli=0'])
        self.assertEqual(r.stdout,'wireless=1\nenabled=1\nconnected=0\ncurrent=\n')
        self.state(wirelessEnable=0,enable=0,linked=True);(self.root/'writes').unlink()
        self.assertEqual(self.run_helper('disconnect').returncode,0);self.assertEqual(self.writes(),[])
    def test_disconnect_tries_wifid_then_wpa_and_restores_auto_when_it_cannot(self):
        self.state(wirelessEnable=1,enable=1,linked=True,profiles=self.PROFILES,cmProfile='Home',cm_noop=True)
        r=self.run_helper('disconnect');self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(self.writes(),['cmConnMode=manual','wpa_cli=status','cmDisconnect=Home','cmDisconnect=','wpa_cli=disconnect','wpa_cli=disconnect','wpa_cli=0'])
        (self.root/'writes').unlink()
        self.state(wirelessEnable=1,enable=1,linked=True,profiles=self.PROFILES,cmProfile='Home',cm_noop=True,wpa_fail=True)
        self.assertEqual(self.run_helper('disconnect').returncode,12)
        # still on and connected: the manual mode must not be left behind
        self.assertEqual(self.writes(),['cmConnMode=manual','wpa_cli=status','cmDisconnect=Home','cmDisconnect=','wpa_cli=disconnect','cmConnMode=auto'])
        (self.root/'wpa_cli').unlink()
        (self.root/'writes').unlink()
        self.state(wirelessEnable=1,enable=1,linked=True,profiles=self.PROFILES,cmProfile='Home')
        self.assertEqual(self.run_helper('disconnect').returncode,0)
        self.assertEqual(self.writes(),['cmConnMode=manual','cmDisconnect=Home'])
    def test_disconnect_names_the_step_it_failed_at(self):
        self.state(wirelessEnable=0,enable=0,unavailable=True)
        self.assertEqual(self.run_helper('disconnect').returncode,10)
        self.state(wirelessEnable=1,enable=1,linked=True,cm_fail=True)
        self.assertEqual(self.run_helper('disconnect').returncode,11)
        # a disabled network cannot come straight back: the drop holds
        self.state(wirelessEnable=1,enable=1,linked=True,profiles=self.PROFILES,cmProfile='Home',rejoin_after=3)
        r=self.run_helper('disconnect');self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(r.stdout,'wireless=1\nenabled=1\nconnected=0\ncurrent=\n')
        self.assertEqual(self.writes(),['cmConnMode=manual','wpa_cli=status','cmDisconnect=Home','wpa_cli=disconnect','wpa_cli=0'])
        (self.root/'writes').unlink()
        # unless wifid overrides the disable itself: the same rejoin past the pin is another exit
        self.state(wirelessEnable=1,enable=1,linked=True,profiles=self.PROFILES,cmProfile='Home',rejoin_after=3,reenable=True)
        r=self.run_helper('disconnect')
        self.assertEqual(r.returncode,17,r.stderr)
        self.assertEqual(self.writes(),['cmConnMode=manual','wpa_cli=status','cmDisconnect=Home','wpa_cli=disconnect','wpa_cli=0','wpa_cli=0','cmConnMode=auto'])
        # without a wpa_cli to pin the drop, the same rejoin is a different exit
        (self.root/'wpa_cli').unlink()
        if (self.root/'writes').exists():(self.root/'writes').unlink()
        self.state(wirelessEnable=1,enable=1,linked=True,profiles=self.PROFILES,cmProfile='Home',rejoin_after=3)
        r=self.run_helper('disconnect')
        self.assertEqual(r.returncode,15,r.stderr)
        self.assertEqual(self.writes(),['cmConnMode=manual','cmDisconnect=Home','cmConnMode=auto'])
        p=self.root/'wpa_cli';p.write_text(FAKE_WPA);p.chmod(0o755)
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
        self.assertEqual(self.writes(),['cmConnMode=auto','ensureConnection=wifi:Home']);self.assertTrue(r.stdout.startswith('wireless=1\n'))
        self.no_password(r)
        # with the radio off, it comes on first, in the order Turn on uses
        self.state(wirelessEnable=0,enable=0,profiles=self.PROFILES);(self.root/'writes').unlink()
        self.assertEqual(self.run_helper('join','Caf\u00e9 \u732b'.encode().hex()).returncode,0)
        self.assertEqual(self.writes(),['wirelessEnable=1','enable=1','cmConnMode=auto','ensureConnection=wifi:Caf\u00e9 \u732b'])
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
        self.assertEqual(self.writes(),['cmConnMode=auto','ensureConnection=wifi:'+name])
        self.assertFalse((self.root/'injected').exists());self.assertFalse((self.root/'i2').exists())
    def test_a_failed_switch_is_reported(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,fail='ensureConnection')
        self.assertNotEqual(self.run_helper('join','Home'.encode().hex()).returncode,0)
    # ---- a scan of the networks near it (2.1.0-beta.2): an explicit read, never a radio change ----
    SCAN_LIST=[{'essid':'Home','signal':4,'signal_max':5,'key_mgmt':'WPA2','known':'yes'},
               {'essid':'Caf\u00e9 \u732b','signal':3,'signal_max':5,'key_mgmt':'WPA3','known':'no'},
               {'essid':'Open','signal':2,'signal_max':5,'key_mgmt':'OPEN','known':'no'},
               {'essid':'Work','signal':1,'signal_max':5,'key_mgmt':'WPA2-ENTERPRISE','known':'no'}]
    def near(self,out):
        rows=[]
        for line in out.splitlines():
            if not line.startswith('near='):continue
            h,sig,mx,sec,known=line[5:].split(',')
            rows.append((bytes.fromhex(h).decode(),int(sig),int(mx),sec,known))
        return rows
    def test_scan_lists_nearby_networks_without_changing_the_radio(self):
        for fmt in ('kv','json','kindle'):
            if (self.root/'writes').exists():(self.root/'writes').unlink()
            self.state(wirelessEnable=1,enable=1,scan_list=self.SCAN_LIST,format=fmt)
            r=self.run_helper('scan');self.assertEqual(r.returncode,0,r.stderr)
            self.assertTrue(r.stdout.startswith('wireless=1\nenabled=1\nconnected=1\n'),fmt)
            self.assertIn('scan=ok',r.stdout.splitlines())
            self.assertEqual(self.near(r.stdout),[
                ('Home',4,5,'wpa2','1'),('Caf\u00e9 \u732b',3,5,'wpa3','0'),
                ('Open',2,5,'open','0'),('Work',1,5,'enterprise','0')],fmt)
            self.assertEqual(r.stdout.splitlines()[-1],'near_end=1')
            self.assertEqual(self.writes(),['scan='],fmt)
            self.assertEqual((self.root/'hash-stdin').read_text().strip(),'{}')
    def test_scan_with_the_radio_off_asks_for_it_and_reads_nothing(self):
        self.state(wirelessEnable=0,enable=0,scan_list=self.SCAN_LIST)
        r=self.run_helper('scan');self.assertEqual(r.returncode,0)
        self.assertEqual(r.stdout.splitlines()[-1],'scan=off')
        self.assertEqual(self.writes(),[])
    def test_a_scan_that_cannot_be_read_or_finish_is_not_a_list(self):
        self.state(wirelessEnable=1,enable=1,scan_list=self.SCAN_LIST,hash_fail=True)
        r=self.run_helper('scan');self.assertNotEqual(r.returncode,0)
        self.assertIn('scan=bad',r.stdout);self.assertNotIn('near=',r.stdout)
        self.state(wirelessEnable=1,enable=1,scan_list=self.SCAN_LIST,scan_fail=True)
        self.assertNotEqual(self.run_helper('scan').returncode,0)
        self.state(wirelessEnable=1,enable=1,scan_list=[{'essid':'bad\\name','signal':1,'signal_max':5,'key_mgmt':'WPA2'}],format='kv')
        self.assertNotEqual(self.run_helper('scan').returncode,0)
    def test_add_creates_a_wpa_profile_and_never_prints_the_passphrase(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES)
        pw='correct horse "battery"'
        r=self.run_helper('add','New net','wpa2',data=pw+'\n')
        self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(self.writes(),['cmConnMode=auto','ensureConnection=wifi:New net'])
        self.assertNotIn(pw,r.stdout);self.assertNotIn(pw,r.stderr)
        h=(self.root/'create-stdin').read_text()
        self.assertIn('essid="New net"',h);self.assertIn('smethod="wpa2"',h);self.assertIn('secured="yes"',h)
        self.assertIn('psk="'+'ab'*32+'"',h);self.assertNotIn(pw,h)
        self.no_password(r)
        for bad in ('short\n','x'*64+'\n','has\ttab\n'):
            self.assertNotEqual(self.run_helper('add','Other','wpa2',data=bad).returncode,0,bad)
    def test_add_takes_a_precomputed_psk_without_wpa_passphrase(self):
        # Grimoire derives the PSK, so a firmware with no wpa_passphrase still adds a WPA/WPA2 network.
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES)
        p=self.root/'wpa_passphrase'
        p.write_text('#!/bin/sh\nd=$(dirname "$0")\ntouch "$d/wpa-passphrase-called"\nexit 1\n')
        p.chmod(0o755)
        psk='cd'*32
        r=self.run_helper('add','New net','wpa2',data=psk+'\n')
        self.assertEqual(r.returncode,0,r.stderr)
        self.assertFalse((self.root/'wpa-passphrase-called').exists())
        h=(self.root/'create-stdin').read_text()
        self.assertIn('essid="New net"',h);self.assertIn('psk="'+psk+'"',h)
        self.assertNotIn(psk,r.stdout);self.assertNotIn(psk,r.stderr)
        self.assertEqual(self.writes(),['cmConnMode=auto','ensureConnection=wifi:New net'])
    def test_add_eap_sends_the_credentials_only_on_stdin(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES)
        user='student@example.edu'; pw='school secret "with quotes"'
        r=self.run_helper('add','School','eap',data=user+'\n'+pw+'\n')
        self.assertEqual(r.returncode,0,r.stderr)
        cmds=(self.root/'wpa-commands').read_text()
        self.assertIn('set_network 0 ssid "School"',cmds)
        self.assertIn('set_network 0 key_mgmt "WPA-EAP WPA-EAP-SHA256"',cmds)
        self.assertIn('set_network 0 eap PEAP',cmds)
        self.assertIn('set_network 0 identity "student@example.edu"',cmds)
        self.assertIn('set_network 0 password "school secret \\"with quotes\\""',cmds)
        self.assertIn('set_network 0 phase2 "auth=MSCHAPV2"',cmds)
        self.assertIn('set_network 0 ieee80211w 1',cmds)
        self.assertIn('enable_network 0',cmds)
        self.assertIn('select_network 0',cmds)
        self.assertIn('save_config',cmds)
        # The interactive session must end itself; EOF alone can leave wpa_cli waiting.
        self.assertTrue(cmds.rstrip().endswith('quit'),cmds)
        self.assertEqual(self.writes().count('wpa_cli=add_network'),1)
        for text in (r.stdout,r.stderr):
            self.assertNotIn(user,text);self.assertNotIn(pw,text)
        argv=(self.root/'wpa-argv').read_text()
        self.assertNotIn(user,argv);self.assertNotIn(pw,argv)
    def test_add_eap_reads_crlf_prompts(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,wpa_prompt='cr')
        r=self.run_helper('add','School','eap',data='student@example.edu\nsecret\n')
        self.assertEqual(r.returncode,0,r.stderr)
        self.assertNotIn('student@example.edu',r.stdout);self.assertNotIn('secret',r.stdout)
    def test_add_eap_falls_back_when_the_supplicant_lacks_sha256(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,cap_key_mgmt='WPA-PSK WPA-EAP NONE')
        r=self.run_helper('add','School','eap',data='student@example.edu\nsecret\n')
        self.assertEqual(r.returncode,0,r.stderr)
        cmds=(self.root/'wpa-commands').read_text()
        self.assertIn('set_network 0 key_mgmt WPA-EAP\n',cmds)
        self.assertNotIn('WPA-EAP-SHA256',cmds)
        self.assertIn('set_network 0 ieee80211w 1',cmds)
    def test_add_wpa3_sends_the_passphrase_only_on_stdin_and_uses_sae(self):
        # The fake prints the real interactive form ("> OK"), which the helper must count.
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES)
        pw='correct horse wifi'
        r=self.run_helper('add','New 3','wpa3',data=pw+'\n')
        self.assertEqual(r.returncode,0,r.stderr)
        cmds=(self.root/'wpa-commands').read_text()
        self.assertIn('set_network 0 ssid "New 3"',cmds)
        self.assertIn('set_network 0 key_mgmt SAE',cmds)
        # SAE needs the passphrase itself, not the PSK wpa_passphrase derives from it.
        self.assertIn('set_network 0 psk "correct horse wifi"',cmds)
        self.assertIn('set_network 0 ieee80211w 2',cmds)
        self.assertIn('enable_network 0',cmds)
        self.assertIn('select_network 0',cmds)
        self.assertIn('save_config',cmds)
        self.assertFalse((self.root/'create-stdin').exists(),'WPA3 never goes through wifid createProfile')
        for text in (r.stdout,r.stderr):
            self.assertNotIn(pw,text)
        argv=(self.root/'wpa-argv').read_text()
        self.assertNotIn(pw,argv)
        self.assertEqual(self.run_helper('add','New 3','wpa3',data='short\n').returncode,2)
        self.assertEqual(self.run_helper('add','New 3','wpa3',data='x'*64+'\n').returncode,2)
        self.assertEqual(self.run_helper('add','New 3','wpa3',data='has\ttab\n').returncode,2)
    def test_add_wpa3_names_a_supplicant_without_sae(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,cap_key_mgmt='WPA-PSK WPA-EAP WPA-EAP-SHA256')
        r=self.run_helper('add','New 3','wpa3',data='password123\n')
        self.assertEqual(r.returncode,28)
        self.assertFalse((self.root/'wpa-commands').exists())
        self.assertNotIn('password123',r.stdout);self.assertNotIn('password123',r.stderr)
    def test_add_eap_names_the_step_it_failed_at_and_rejects_bad_values(self):
        self.state(wirelessEnable=1,enable=1)
        p=self.root/'wpa_cli';p.unlink()
        r=self.run_helper('add','School','eap',data='student@example.edu\nsecret\n')
        self.assertEqual(r.returncode,26)
        self.assertNotIn('student@example.edu',r.stdout);self.assertNotIn('secret',r.stdout)
        p.write_text(FAKE_WPA);p.chmod(0o755)
        self.state(wirelessEnable=1,enable=1,wpa_fail_command='set_network 0 password')
        r=self.run_helper('add','School','eap',data='student@example.edu\nsecret\n')
        self.assertEqual(r.returncode,27)
        self.assertNotIn('student@example.edu',r.stdout);self.assertNotIn('secret',r.stdout)
        # Nothing invalid reaches the supplicant at all.
        (self.root/'wpa-commands').unlink(missing_ok=True)
        for data in ('\nsecret\n','user\n\n','x'*129+'\nsecret\n','user\n'+'y'*129+'\n'):
            self.assertEqual(self.run_helper('add','School','eap',data=data).returncode,2,data)
        self.assertFalse((self.root/'wpa-commands').exists())
    def test_add_names_the_step_it_failed_at(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES)
        # wpa_passphrase broken: no PSK, no profile written, no secret out
        (self.root/'wpa_passphrase').write_text('#!/bin/sh\nexit 1\n');(self.root/'wpa_passphrase').chmod(0o755)
        r=self.run_helper('add','New net','wpa2',data='correct horse\n')
        self.assertEqual(r.returncode,18,r.stderr)
        self.assertFalse((self.root/'create-stdin').exists())
        self.assertNotIn('correct horse',r.stdout);self.assertNotIn('correct horse',r.stderr)
        (self.root/'wpa_passphrase').write_text('#!/usr/bin/env python3\nprint("network={\\n}")\n');(self.root/'wpa_passphrase').chmod(0o755)
        self.assertEqual(self.run_helper('add','New net','wpa2',data='correct horse\n').returncode,18)
        p=self.root/'wpa_passphrase'
        p.write_text('#!/usr/bin/env python3\nimport sys\npw=sys.stdin.readline().rstrip("\\n")\nprint("network={")\nprint("\\tssid=\\""+sys.argv[1]+"\\"")\nprint("\\t#psk=\\""+pw+"\\"")\nprint("\\tpsk="+"ab"*32)\nprint("}")\n');p.chmod(0o755)
        # wifid refuses the profile
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,hash_fail=True)
        self.assertEqual(self.run_helper('add','New net','wpa2',data='correct horse\n').returncode,19)
        # created, then no read came back
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,late_fail=True)
        self.assertEqual(self.run_helper('add','New net','wpa2',data='correct horse\n').returncode,20)
    def test_add_open_needs_no_passphrase_and_a_hostile_name_is_only_data(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES)
        name='a"; touch injected; $(touch i2)'
        r=self.run_helper('add',name,'open')
        self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(self.writes(),['cmConnMode=auto','ensureConnection=wifi:'+name])
        h=(self.root/'create-stdin').read_text()
        self.assertIn('smethod="open"',h);self.assertIn('secured="no"',h);self.assertNotIn('psk=',h)
        self.assertFalse((self.root/'injected').exists());self.assertFalse((self.root/'i2').exists())
    def test_delete_forgets_one_saved_profile(self):
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,current={'essid':'Home','netid':1})
        r=self.run_helper('delete','Open')
        self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(self.writes(),['deleteProfile=Open'])
        self.assertNotIn('Open',self.saved(r.stdout))
        self.assertEqual(self.saved(r.stdout),['Home','Caf\u00e9 \u732b'])
        self.no_password(r)
    def test_delete_names_the_step_it_failed_at(self):
        self.state(wirelessEnable=0,enable=0,unavailable=True)
        self.assertEqual(self.run_helper('delete','Open').returncode,21)
        # the netid write is refused too: the profile is still saved and the exit says so
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,fail='deleteProfile')
        self.assertEqual(self.run_helper('delete','Open').returncode,25)
        # no netid in profileData or netidValue: the integer route is not possible
        self.state(wirelessEnable=1,enable=1,profiles=[{'essid':'Open','secured':'no'}],delete_string_fail=True)
        self.assertEqual(self.run_helper('delete','Open').returncode,24)
        # gone, but the list cannot be read back to confirm it
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,hash_fail=True)
        self.assertEqual(self.run_helper('delete','Open').returncode,23)
    def test_delete_tries_wifid_by_name_then_by_netid_then_the_supplicant(self):
        # the name is refused (the Scribe's wifid takes an integer): the profile's netid removes it
        for fmt in ('kv','json','kindle'):
            if (self.root/'writes').exists():(self.root/'writes').unlink()
            self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,current={'essid':'Home','netid':1},
                       delete_string_fail=True,format=fmt)
            r=self.run_helper('delete','Caf\u00e9 \u732b');self.assertEqual(r.returncode,0,(fmt,r.stderr))
            self.assertEqual(self.writes(),['deleteProfile=Caf\u00e9 \u732b','deleteProfile=2'],fmt)
            self.assertEqual(self.saved(r.stdout),['Home','Open'],fmt)
            self.no_password(r)
        # both lipc routes refused, the supplicant holds the network
        self.state(wirelessEnable=1,enable=1,profiles=self.PROFILES,fail='deleteProfile',
                   wpa_networks=[{'id':3,'ssid':'Open'}])
        r=self.run_helper('delete','Open');self.assertEqual(r.returncode,0,r.stderr)
        self.assertNotIn('Open',self.saved(r.stdout))
        self.assertIn('wpa_cli=save_config',self.writes())
        self.no_password(r)
    def test_delete_reads_the_netid_from_netidValue_when_profileData_has_none(self):
        self.state(wirelessEnable=1,enable=1,profiles=[{'essid':'Open','secured':'no'}],
                   netid_values=[{'essid':'Open','netid':7}],delete_string_fail=True)
        r=self.run_helper('delete','Open');self.assertEqual(r.returncode,0,r.stderr)
        self.assertIn('deleteProfile=7',self.writes())
        self.assertEqual(self.saved(r.stdout),[])
        self.no_password(r)
    def test_scan_names_wpa3_enterprise_192_bit_apart(self):
        self.state(wirelessEnable=1,enable=1,scan_list=[
            {'essid':'SAE','signal':5,'signal_max':5,'key_mgmt':'SAE','known':'no'},
            {'essid':'WPA3-EAP','signal':4,'signal_max':5,'key_mgmt':'WPA-EAP-SHA256','known':'no'},
            {'essid':'192','signal':3,'signal_max':5,'key_mgmt':'WPA-EAP-SUITE-B-192','known':'no'},
            {'essid':'Mixed','signal':2,'signal_max':5,'key_mgmt':'WPA-EAP-SHA256 WPA-EAP-SUITE-B-192','known':'no'},
        ])
        r=self.run_helper('scan');self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(self.near(r.stdout),[
            ('SAE',5,5,'wpa3','0'),('WPA3-EAP',4,5,'enterprise','0'),('192',3,5,'enterprise192','0'),
            ('Mixed',2,5,'enterprise','0')])
    def test_a_hostile_name_is_data_in_a_scan_too(self):
        name="$(touch injected) `touch i2`;x"
        self.state(wirelessEnable=1,enable=1,scan_list=[{'essid':name,'signal':4,'signal_max':5,'key_mgmt':'WPA2','known':'no'}])
        r=self.run_helper('scan');self.assertEqual(r.returncode,0,r.stderr)
        self.assertEqual(self.near(r.stdout),[ (name,4,5,'wpa2','0') ])
        self.assertFalse((self.root/'injected').exists());self.assertFalse((self.root/'i2').exists())
if __name__=='__main__':unittest.main()
