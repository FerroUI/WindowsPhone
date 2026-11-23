import assert from 'node:assert/strict';
import test from 'node:test';
import {readFile} from 'node:fs/promises';
import vm from 'node:vm';
import {notificationPermission,requestNotificationPermission,showSystemNotification} from '../crates/phone-browser/wwwroot/system-notifications.js';
import {readStorageEnvironment,requestPersistentStorage} from '../crates/phone-browser/wwwroot/storage-environment.js';
import {observeEnvironment} from '../crates/phone-browser/wwwroot/environment.js';
import {createMediaSession} from '../crates/phone-browser/wwwroot/media-session.js';

const settle = () => new Promise(resolve => setImmediate(resolve));

test('notifications separate observation and delivery from explicit permission requests',async()=>{
  let prompts=0,shown=0;
  class Notification extends EventTarget {
    static permission='default';
    static requestPermission(){prompts++;this.permission='granted';return Promise.resolve('granted');}
    constructor(){super();shown++;}
  }
  const platform={Notification};
  assert.equal(notificationPermission(platform),'prompt');
  assert.equal(await showSystemNotification('alarm','meeting',platform),false);
  assert.equal(prompts,0);assert.equal(shown,0);
  const request=requestNotificationPermission(platform);
  assert.equal(prompts,1,'the permission API must receive the synchronous user gesture');
  assert.equal(await request,'granted');
  assert.equal(await showSystemNotification('alarm','meeting',platform),true);
  assert.equal(shown,1);
  Notification.permission='denied';
  assert.equal(await requestNotificationPermission(platform),'denied');
  assert.equal(prompts,1,'a blocked permission must not be requested again');
  assert.equal(notificationPermission({}),'unsupported');
});

test('persistent notifications use the active app worker and never wait for registration readiness',async()=>{
  let calls=0;
  class Notification {static permission='granted';constructor(){throw Error('mobile constructor unavailable');}}
  const platform={Notification,navigator:{serviceWorker:{
    get ready(){throw Error('must not wait indefinitely for a service worker');},
    getRegistration:async scope=>{assert.ok(scope.endsWith('/wwwroot/'));return {active:{},showNotification:async(title,options)=>{calls++;assert.equal(title,'Reminder');assert.equal(options.data.phone,true);assert.equal(options.body,'Start now');}};},
  }}};
  assert.equal(await showSystemNotification('Reminder','Start now',platform),true);assert.equal(calls,1);
  platform.navigator.serviceWorker.getRegistration=async()=>undefined;
  await assert.rejects(showSystemNotification('Reminder','Start now',platform),/constructor unavailable/);
});

test('notification activation only focuses an app-scoped window or opens the fixed app URL',async()=>{
  const handlers=new Map(),calls=[];
  const scope='https://example.test/WindowsPhone/';
  let clients=[{url:'https://example.test/WindowsPhone-other/',focus:()=>calls.push('wrong')}];
  const worker={registration:{scope},addEventListener:(name,handler)=>handlers.set(name,handler),clients:{matchAll:async()=>clients,openWindow:url=>calls.push(url)}};
  const source=await readFile(new URL('../crates/phone-browser/wwwroot/app-sw.js',import.meta.url),'utf8');
  vm.runInNewContext(source.replace('__PRECACHE__','[]'),{self:worker,URL});
  const click=async data=>{let pending;handlers.get('notificationclick')({notification:{data,close:()=>{}},waitUntil:promise=>pending=promise});await pending;};
  await click({phone:true,url:'https://outside.test/'});assert.deepEqual(calls,[scope]);
  clients=[{url:`${scope}?native=1`,focus:()=>calls.push('focused')}];
  await click({phone:true});assert.equal(calls.at(-1),'focused');
  const count=calls.length;await click({phone:false});assert.equal(calls.length,count);
});

test('storage observations preserve independent failures and explicit persistence rejection',async()=>{
  let prompts=0;
  const storage={estimate:async()=>({usage:1024.9,quota:8192}),persisted:async()=>{throw Error('blocked');},persist:()=>{prompts++;return Promise.resolve(false);}};
  assert.deepEqual(await readStorageEnvironment(storage),{storage_usage_bytes:1024,storage_quota_bytes:8192,storage_persisted:null,persistent_storage_supported:true});
  assert.equal(prompts,0);
  const request=requestPersistentStorage(storage);assert.equal(prompts,1);assert.equal(await request,false);
  assert.deepEqual(await readStorageEnvironment({estimate:()=>({usage:Infinity,quota:-1}),persisted:async()=>true}),{storage_usage_bytes:null,storage_quota_bytes:null,storage_persisted:true,persistent_storage_supported:false});
  await assert.rejects(requestPersistentStorage({}),/unavailable/);
});

test('host observations deduplicate facts and clean up pending sensors and event listeners',async()=>{
  const platform=new EventTarget();platform.document=new EventTarget();platform.document.hidden=false;
  let completeBattery;let permissionPrompts=0;
  platform.Notification={permission:'default',requestPermission:()=>{permissionPrompts++;}};
  platform.navigator={onLine:true,getBattery:()=>new Promise(resolve=>{completeBattery=resolve;}),storage:{estimate:async()=>({usage:5,quota:10}),persisted:async()=>false}};
  const states=[];const stop=observeEnvironment({setEnvironment:json=>states.push(JSON.parse(json))},platform);
  await settle();
  assert.equal(states.at(-1).storage_usage_bytes,5);assert.equal(states.at(-1).notification_permission,'prompt');
  assert.equal(permissionPrompts,0);
  const count=states.length;platform.dispatchEvent(new Event('online'));assert.equal(states.length,count);
  platform.navigator.onLine=false;platform.dispatchEvent(new Event('offline'));assert.equal(states.at(-1).online,false);
  stop();const stopped=states.length;
  const battery=new EventTarget();battery.level=.4;battery.charging=true;completeBattery(battery);
  platform.navigator.onLine=true;platform.dispatchEvent(new Event('online'));
  await settle();battery.dispatchEvent(new Event('levelchange'));assert.equal(states.length,stopped);
});

function fakeSession(){
  return {actions:new Map(),metadata:null,playbackState:'none',positions:[],
    setActionHandler(name,handler){if(name==='seekforward')throw Error('unsupported action');this.actions.set(name,handler);},
    setPositionState(value){this.positions.push(value);},
  };
}
function fakePlayer(){return Object.assign(new EventTarget(),{duration:120,currentTime:15,playbackRate:1,paused:false,ended:false});}

test('media session routes hardware controls and clamps positions while tolerating partial host support',async()=>{
  const session=fakeSession(),player=fakePlayer(),commands=[],errors=[];
  const adapter=createMediaSession({session,Metadata:class{constructor(value){Object.assign(this,value);}},control:action=>{commands.push(action);if(action==='play')return Promise.reject(Error('play blocked'));},report:error=>errors.push(error)});
  adapter.attach(player,{title:'Song',artist:'Artist'});
  assert.equal(session.metadata.title,'Song');assert.equal(session.playbackState,'playing');
  session.actions.get('seekbackward')({seekOffset:30});assert.equal(player.currentTime,0);
  session.actions.get('seekto')({seekTime:900});assert.equal(player.currentTime,120);
  session.actions.get('seekto')({seekTime:NaN});assert.equal(player.currentTime,120);
  session.actions.get('nexttrack')();session.actions.get('previoustrack')();session.actions.get('play')();
  await settle();assert.deepEqual(commands,['next','previous','play']);assert.deepEqual(errors,['play blocked']);
  player.paused=true;player.dispatchEvent(new Event('pause'));assert.equal(session.playbackState,'paused');
  player.duration=NaN;player.dispatchEvent(new Event('durationchange'));assert.equal(session.positions.at(-1),undefined);
  adapter.clear();assert.equal(session.metadata,null);assert.equal(session.playbackState,'none');assert.equal(session.actions.get('nexttrack'),null);
});

test('replacing a media player detaches the old element and stop clears every handler',()=>{
  const session=fakeSession(),old=fakePlayer(),next=fakePlayer();next.currentTime=65;
  const adapter=createMediaSession({session,control:()=>{}});
  adapter.attach(old,{});adapter.attach(next,{});
  const count=session.positions.length;old.dispatchEvent(new Event('timeupdate'));assert.equal(session.positions.length,count);
  next.dispatchEvent(new Event('timeupdate'));assert.equal(session.positions.at(-1).position,65);
  adapter.dispose();const cleared=session.positions.length;next.dispatchEvent(new Event('timeupdate'));assert.equal(session.positions.length,cleared);
});
