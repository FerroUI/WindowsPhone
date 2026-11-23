import assert from 'node:assert/strict';
import test from 'node:test';
import {mediaSources,prepareBackup} from '../crates/phone-browser/wwwroot/backup-format.js';

const phone=()=>({version:1,photos:[{source:'media:photo'}],tracks:[{source:'media:song'}],settings:{wallpaper:'media:photo',lock_wallpaper:'media:lock'}});
const asset=(key,kind='image',raw='abcd')=>({key,name:'Photo 11/23/2026',mime:kind==='image'?'image/jpeg':'audio/mpeg',kind,data:btoa(raw)});
const envelope=()=>({format:'ferroui-windows-phone',version:1,phone:phone(),assets:[asset('media:photo'),asset('media:song','audio'),asset('media:lock')]});
const validate=json=>{const value=JSON.parse(json);assert.ok(Array.isArray(value.photos));assert.ok(Array.isArray(value.tracks));};

test('backups include independent lock wallpaper and restore immutable assets under fresh keys',()=>{
  assert.deepEqual([...mediaSources(phone())],['media:photo','media:song','media:lock']);
  const input=envelope();let next=0;
  const restored=prepareBackup(input,validate,()=>`media:fresh${++next}`);
  const data=JSON.parse(restored.json);
  assert.equal(data.photos[0].source,'media:fresh1');assert.equal(data.settings.wallpaper,'media:fresh1');assert.equal(data.settings.lock_wallpaper,'media:fresh3');
  assert.equal(input.phone.photos[0].source,'media:photo','preparation must not change the previous phone');
  assert.equal(restored.missing,0);assert.equal(restored.assets.length,3);
  assert.deepEqual([...new Uint8Array(restored.assets[0].bytes)],[97,98,99,100]);
  assert.equal(restored.assets[0].name,'Photo 11/23/2026','legacy camera labels remain display metadata');
});

test('backup preparation rejects duplicate keys, path-like identifiers and malformed payloads atomically',()=>{
  for(const change of [
    value=>value.assets.push(value.assets[0]),
    value=>value.assets.push(asset('media:unused')),
    value=>value.assets[0].key='media:../escape',
    value=>value.assets[0].data='YQ===',
    value=>value.assets[0].data='YR==',
    value=>value.assets[0].data='',
    value=>value.assets[0].data='YQ==\n',
    value=>value.assets[0].name='',
    value=>value.assets[0].kind='video',
    value=>value.assets[0].mime='image/jpeg\nother',
    value=>value.phone.tracks[0].source='media:photo',
    value=>value.version=2,
    value=>value.phone={},
  ]){
    const input=envelope();change(input);assert.throws(()=>prepareBackup(input,validate,()=>`media:fresh${Math.random().toString().slice(2)}`));
  }
  assert.throws(()=>prepareBackup(envelope(),validate,()=> 'media:photo'),/fresh media identifier/);
  assert.throws(()=>prepareBackup(envelope(),validate,()=> 'media:same'),/fresh media identifier/);
});

test('large canonical base64 is validated without recursive regexp matching',()=>{
  const input=envelope();input.assets=[asset('media:photo','image','a'.repeat(1024*1024+1))];
  const restored=prepareBackup(input,validate,()=> 'media:large');
  assert.equal(restored.assets[0].bytes.byteLength,1024*1024+1);
  assert.equal(new Uint8Array(restored.assets[0].bytes).at(-1),97);
});

test('backup preparation preserves explicit missing assets and bounded legacy phone documents',()=>{
  const input=envelope();input.assets.pop();let next=0;
  const restored=prepareBackup(input,validate,()=>`media:next${++next}`);
  assert.equal(restored.missing,1);assert.equal(JSON.parse(restored.json).settings.lock_wallpaper,'media:lock');
  const legacy=prepareBackup(phone(),validate);assert.equal(legacy.assets.length,0);assert.equal(legacy.missing,3);
  const tooLarge=envelope();tooLarge.assets[0].data='A'.repeat(Math.ceil(16*1024*1024/3)*4+4);
  assert.throws(()=>prepareBackup(tooLarge,validate),/oversized/);
});
