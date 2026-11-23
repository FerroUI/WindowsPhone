import assert from 'node:assert/strict';
import test from 'node:test';

const flush=()=>new Promise(resolve=>setImmediate(resolve));
let generation=0;
function events(){
  const handlers=new Map();
  return {hidden:false,handlers,
    addEventListener(name,callback){const list=handlers.get(name)||[];list.push(callback);handlers.set(name,list);},
    fire(name){for(const callback of handlers.get(name)||[])callback();}};
}

async function browser(t,{state='running',vibrationThrows=false}={}){
  const saved=new Map(['window','document','navigator','setInterval','clearInterval']
    .map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
  const document=events(),window=events(),intervals=new Map(),pending=[];
  const counts={contexts:0,tones:0,stops:0};
  const settings={state};let timer=0,api;
  class AudioContext {
    constructor(){counts.contexts++;this.state=settings.state;this.currentTime=0;this.destination={};}
    resume(){return this.state==='running'?Promise.resolve():new Promise(resolve=>pending.push(()=>{this.state='running';resolve();}));}
    close(){this.state='closed';return Promise.reject(Error('closed during teardown'));}
    createOscillator(){counts.tones++;return {frequency:{value:0},connect:target=>target,start(){},stop(){counts.stops++;},disconnect(){}};}
    createGain(){return {gain:{setValueAtTime(){},linearRampToValueAtTime(){}},connect:target=>target,disconnect(){}};}
  }
  window.AudioContext=AudioContext;
  const globals={window,document,navigator:{vibrate(){if(vibrationThrows)throw Error('vibration unavailable');return true;}},
    setInterval:callback=>{const id=++timer;intervals.set(id,callback);return id;},clearInterval:id=>intervals.delete(id)};
  for(const [key,value] of Object.entries(globals))Object.defineProperty(globalThis,key,{configurable:true,writable:true,value});
  t.after(async()=>{
    try{api?.setAlarmAlert({active:false,volume:0,vibration:false});window.fire('pagehide');await flush();}
    finally{for(const [key,descriptor] of saved){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}}
  });
  // A fresh production module isolates its one-context/one-timer state per test.
  api=await import(`../crates/phone-browser/wwwroot/alarm-alert.js?probe=${generation++}`);
  return {api,document,window,intervals,pending,counts,settings};
}

test('alarm audio isolates vibration failures, attaches once and cleans up dismissal',async t=>{
  const f=await browser(t,{vibrationThrows:true});
  f.api.attachAlarmAudio(()=>{});f.api.attachAlarmAudio(()=>{});
  assert.equal(f.document.handlers.get('pointerdown').length,1);
  assert.equal(f.window.handlers.get('pagehide').length,1);
  f.api.setAlarmAlert({active:true,volume:8,vibration:true});
  assert.equal(f.counts.tones,3);assert.equal(f.intervals.size,1);
  f.document.hidden=true;f.document.fire('visibilitychange');
  for(const pulse of f.intervals.values())pulse();
  assert.equal(f.counts.tones,3,'hidden pages cannot create more alarm voices');
  f.document.hidden=false;f.document.fire('visibilitychange');
  assert.equal(f.counts.tones,6);
  f.api.setAlarmAlert({active:false,volume:0,vibration:false});
  assert.equal(f.intervals.size,0);assert(f.counts.stops>=12);
  f.window.fire('pagehide');await flush();
});

test('late audio resume cannot restart a hidden page and restoration keeps one timer',async t=>{
  const f=await browser(t,{state:'suspended'});f.api.attachAlarmAudio(()=>{});
  f.api.setAlarmAlert({active:true,volume:8,vibration:false});
  f.document.fire('pointerdown');assert.equal(f.pending.length,2);
  f.window.fire('pagehide');assert.equal(f.intervals.size,0);
  for(const complete of f.pending.splice(0))complete();
  await flush();f.document.fire('visibilitychange');
  assert.deepEqual(f.counts,{contexts:1,tones:0,stops:0});
  f.settings.state='running';f.window.fire('pageshow');
  assert.equal(f.counts.contexts,2);assert.equal(f.counts.tones,3);assert.equal(f.intervals.size,1);
  f.window.fire('pageshow');assert.equal(f.intervals.size,1);assert.equal(f.counts.tones,3);
});
