import test from 'node:test';
import assert from 'node:assert/strict';
import {mapTile,mapTileUrl} from '../crates/phone-browser/wwwroot/map-tiles.js';

const request={request_id:1,zoom:2,x:1,y:2};
// Contract fixture, not a decoder/render fixture. The retained image decoder
// separately rejects a truncated PNG before presenting pixels.
function png(){const bytes=new Uint8Array(33);bytes.set([137,80,78,71,13,10,26,10]);bytes[11]=13;bytes.set([73,72,68,82],12);const v=new DataView(bytes.buffer);v.setUint32(16,256);v.setUint32(20,256);return bytes;}
function fixture(){let pending=true;const completed=[],errors=[];return {completed,errors,cancel(){pending=false;},api:{isHttpTaskPending:()=>pending,completeHttpTask(id,bytes){completed.push(bytes);pending=false;return true;},failHttpTask(id,error){errors.push(error);pending=false;return true;}}};}
function cacheStorage(){const entries=new Map();const url=key=>typeof key==='string'?key:key.url;const cache={async match(key){return entries.get(url(key))?.clone();},async put(key,response){entries.set(url(key),response.clone());},async delete(key){return entries.delete(url(key));},async keys(){return [...entries.keys()].map(url=>({url}));}};return {entries,async open(){return cache;}};}

test('viewed map tiles validate coordinates and keep caching, referrer and anonymous fetch semantics',async()=>{
  for(const invalid of [{zoom:255,x:0,y:0},{zoom:-1,x:0,y:0},{zoom:2,x:4,y:0},{zoom:2,x:0,y:4},{zoom:2,x:.5,y:0}])assert.throws(()=>mapTileUrl(invalid));
  const f=fixture(),storage=cacheStorage();
  await mapTile(f.api,request,{storage,now:()=>1000,fetcher:async(url,options)=>{assert.equal(url,'https://tile.openstreetmap.org/2/1/2.png');assert.equal(options.cache,'default');assert.equal(options.credentials,'omit');assert.equal(options.redirect,'error');assert.equal(options.referrerPolicy,'strict-origin-when-cross-origin');return new Response(png());}});
  assert.equal(f.errors.length,0);assert.equal(f.completed.length,1);assert.equal(storage.entries.size,1);
  const cached=fixture();await mapTile(cached.api,request,{storage,now:()=>2000,fetcher:()=>assert.fail('fresh cached tile was fetched again')});
  assert.equal(cached.completed.length,1);
});
test('map cache honors freshness, age and no-store instead of imposing seven days',async()=>{
  const storage=cacheStorage();let now=1000,calls=0;
  const fetcher=async()=>{calls++;return new Response(png(),{headers:{'Cache-Control':'max-age=60','Age':'10'}});};
  await mapTile(fixture().api,request,{storage,now:()=>now,fetcher});
  now=50999;await mapTile(fixture().api,request,{storage,now:()=>now,fetcher});assert.equal(calls,1);
  now=51000;await mapTile(fixture().api,request,{storage,now:()=>now,fetcher});assert.equal(calls,2);
  now=200000;const f=fixture();await mapTile(f.api,request,{storage,now:()=>now,fetcher:async()=>new Response(png(),{headers:{'Cache-Control':'no-store'}})});
  assert.equal(f.completed.length,1);assert.equal(storage.entries.size,0);
});
test('map tasks reject oversized or invalid images and suppress stale completions',async()=>{
  for(const response of [()=>new Response('not PNG'),()=>new Response(png(),{headers:{'Content-Length':'524289'}}),()=>new Response(new Uint8Array(524289))]){
    const f=fixture();await mapTile(f.api,request,{storage:null,fetcher:async()=>response()});assert.equal(f.completed.length,0);assert.equal(f.errors.length,1);
  }
  const f=fixture(),storage=cacheStorage();await mapTile(f.api,request,{storage,fetcher:async()=>{f.cancel();return new Response(png());}});
  assert.equal(f.completed.length,0);assert.equal(f.errors.length,0);assert.equal(storage.entries.size,0);
});
test('viewed cache retains fresh tiles at capacity and reclaims only expired entries',async()=>{
  const storage=cacheStorage(),cache=await storage.open();
  for(let index=0;index<2048;index++)await cache.put(`https://tile.openstreetmap.org/12/${index}/0.png`,new Response(png(),{headers:{'X-Phone-Map-Expires':'100000','X-Phone-Map-Size':'33'}}));
  const full=fixture();await mapTile(full.api,request,{storage,now:()=>1000,fetcher:async()=>new Response(png())});
  assert.equal(full.completed.length,0);assert.match(full.errors[0],/full/);assert.equal(storage.entries.size,2048);
  const expired=fixture();await mapTile(expired.api,request,{storage,now:()=>100000,fetcher:async()=>new Response(png())});
  assert.equal(expired.completed.length,1);assert.equal(storage.entries.size,1);
});
