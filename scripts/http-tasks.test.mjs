import test from 'node:test';
import assert from 'node:assert/strict';
import {httpGet,publicHttpsUrl} from '../crates/phone-browser/wwwroot/http-tasks.js';
function fixture() {
  let pending=true; const completed=[],errors=[];
  return {completed,errors,cancel(){pending=false;},api:{isHttpTaskPending:()=>pending,completeHttpTask(id,bytes){completed.push([...bytes]);pending=false;return true;},failHttpTask(id,error){errors.push(error);pending=false;return true;}}};
}
const request={request_id:1,url:'https://example.org/feed',max_bytes:32};
const response=(...chunks)=>new Response(new ReadableStream({start(controller){for(const chunk of chunks)controller.enqueue(new Uint8Array(chunk));controller.close();}}));
test('SDK HTTP validates public HTTPS, reads bounded chunks, and omits credentials',async()=>{
  for(const url of ['http://example.org','https://user:password@example.org','https://127.0.0.1','https://[::1]','https://localhost','https://a.local','https://example.org:8080'])assert.throws(()=>publicHttpsUrl(url));
  const f=fixture();
  await httpGet(f.api,request,async(url,options)=>{assert.equal(url,request.url);assert.equal(options.credentials,'omit');assert.equal(options.redirect,'error');return response([1,2],[3]);});
  assert.deepEqual(f.completed,[[1,2,3]]);assert.deepEqual(f.errors,[]);
});
test('SDK HTTP rejects response overflow and stale completions',async()=>{
  const oversized=fixture();await httpGet(oversized.api,{...request,max_bytes:2},async()=>response([1],[2,3]));
  assert.equal(oversized.completed.length,0);assert.match(oversized.errors[0],/size limit/);
  const stale=fixture();await httpGet(stale.api,request,async()=>{stale.cancel();return response([1]);});
  assert.deepEqual(stale.completed,[]);assert.deepEqual(stale.errors,[]);
  const advertised=fixture();await httpGet(advertised.api,request,async()=>new Response('x',{headers:{'Content-Length':'10000'}}));
  assert.equal(advertised.completed.length,0);assert.match(advertised.errors[0],/size limit/);
});
test('SDK HTTP reports fetch/CORS failures and invalid options without completion',async()=>{
  const f=fixture();await httpGet(f.api,request,async()=>{throw new TypeError('Failed to fetch');});assert.match(f.errors[0],/cross-origin/);
  const invalid=fixture();let called=false;await httpGet(invalid.api,{...request,max_bytes:0},async()=>{called=true;});assert.equal(called,false);assert.equal(invalid.completed.length,0);
});
