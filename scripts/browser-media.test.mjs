import assert from 'node:assert/strict';
import test from 'node:test';

test('media requests cannot revive superseded playback or corrupt controls after a failed load',async()=>{
  const players=[];
  class Player extends EventTarget {
    constructor(){super();players.push(this);}
    src='';paused=true;ended=false;duration=120;currentTime=0;playbackRate=1;volume=1;
    async play(){this.paused=false;this.onplay?.();this.dispatchEvent(new Event('play'));}
    pause(){this.paused=true;this.onpause?.();this.dispatchEvent(new Event('pause'));}
    removeAttribute(name){if(name==='src')this.src='';}
    load(){}
  }
  const originals=new Map(['Audio','navigator','indexedDB','document','MediaMetadata'].map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
  const assign=(key,value)=>Object.defineProperty(globalThis,key,{value,configurable:true,writable:true});
  const assets=new Map(['a','b','c'].map(key=>[`media:${key}`,{key:`media:${key}`,name:`Song ${key}.mp3`,mime:'audio/mpeg',kind:'audio',bytes:new Uint8Array([1,2,3]).buffer}]));
  const delayed=new Map();let hold;
  const connection={transaction:()=>({objectStore:()=>({get:key=>{
    const request={};const finish=()=>{request.result=assets.get(key);request.onsuccess();};
    if(key===hold){hold=undefined;delayed.set(key,finish);}else queueMicrotask(finish);
    return request;
  }})})};
  assign('Audio',Player);assign('document',{createElement:()=>new Player(),getElementById:()=>surface});
  assign('navigator',{mediaSession:{setActionHandler(){},setPositionState(){}}});
  assign('MediaMetadata',class{constructor(value){Object.assign(this,value);}});
  assign('indexedDB',{open:()=>{const request={};queueMicrotask(()=>{request.result=connection;request.onsuccess();});return request;}});
  const states=[],surface={replaceChildren(){}};
  try{
    const media=await import('../crates/phone-browser/wwwroot/media.js');
    const stream='https://example.org/podcast.mp3';
    const runtime={hostState:()=>JSON.stringify({settings:{volume:15,media_muted:false}}),exportData:()=>JSON.stringify({tracks:[...assets.keys(),stream].map(source=>({source,title:source,artist:'Artist',media_kind:'audio'}))}),mediaChanged:(title,source,playing)=>states.push({title,source,playing}),clearImages:()=>{}};
    media.attachMedia(runtime,()=>{});
    hold='media:a';const older=media.playMedia('media:a',surface);await new Promise(resolve=>setImmediate(resolve));
    await media.playMedia('media:b',surface);delayed.get('media:a')();await older;
    assert.equal(states.at(-1).source,'media:b');assert.equal(states.at(-1).playing,true);
    await assert.rejects(media.playMedia('media:missing',surface),/Import this media/);
    await media.mediaControl('pause');assert.equal(states.at(-1).source,'media:b');assert.equal(states.at(-1).playing,false,'failed loads must keep the current player event binding');
    await media.mediaControl('play');assert.equal(states.at(-1).playing,true,'resuming the same source must keep media state updates');
    hold='media:c';const pending=media.playMedia('media:c',surface);await new Promise(resolve=>setImmediate(resolve));
    await media.mediaControl('pause');delayed.get('media:c')();await pending;
    assert.equal(states.at(-1).source,'media:b');assert.equal(states.at(-1).playing,false,'pause must cancel a still-loading replacement');
    await media.mediaControl('stop');
    await media.mediaControl('restart');
    assert.equal(players[0].currentTime,0);
    assert.equal(states.at(-1).playing,true,'restart must replace stopped event bindings and publish observed play');
    await media.mediaControl('pause');assert.equal(states.at(-1).playing,false,'restarted playback must retain pause callbacks');
    await media.mediaControl('seek:900');assert.equal(players[0].currentTime,120,'seeking clamps to observed duration');
    await assert.rejects(media.mediaControl('seek:NaN'),/Invalid playback position/);
    await assert.rejects(media.mediaControl('seek:-1'),/Invalid playback position/);
    await media.playMedia(stream,surface);assert.equal(players[0].src,stream);assert.equal(states.at(-1).source,stream);
    await assert.rejects(media.playMedia('https://127.0.0.1/private.mp3',surface),/public|HTTPS|address|host/i);
    await assert.rejects(media.playMedia('https://example.org/missing.mp3',surface),/no longer in your library/);
    await media.mediaControl('pause');assert.equal(states.at(-1).source,stream);assert.equal(states.at(-1).playing,false);
    await media.restoreAssets(runtime);assert.deepEqual(states.at(-1),{title:'',source:'',playing:false});
  }finally{
    for(const [key,descriptor]of originals){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}
  }
});
