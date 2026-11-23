import assert from 'node:assert/strict';
import test from 'node:test';
import {documentFiles,shareDocuments} from '../crates/phone-browser/wwwroot/document-share.js';

const runtime = {exportOffice:id=>JSON.stringify({name:`Document ${id}.docx`,mime:'application/vnd.openxmlformats-officedocument.wordprocessingml.document',bytes:[80,75,3,4]})};

test('document sharing prepares actual files once and retains synchronous user activation',async()=>{
  let calls=0;let invoked=false;
  const files=documentFiles(runtime,[1,1,2]);
  assert.equal(files.length,2);assert.equal(files[0].name,'Document 1.docx');
  assert.deepEqual([...new Uint8Array(await files[0].arrayBuffer())],[80,75,3,4]);
  const result=shareDocuments(runtime,[1,2],{canShare:({files})=>files.length===2,share:({files})=>{calls++;invoked=true;assert.equal(files[1].name,'Document 2.docx');return Promise.resolve();}});
  assert.equal(invoked,true);assert.equal(await result,true);assert.equal(calls,1);
});

test('failed preparation, unsupported sharing and cancellation do not report success',async()=>{
  let calls=0;const platform={canShare:()=>true,share:()=>{calls++;return Promise.resolve();}};
  await assert.rejects(shareDocuments({exportOffice:id=>{if(id===2)throw Error('missing');return runtime.exportOffice(id);}},[1,2],platform),/missing/);
  assert.equal(calls,0);
  await assert.rejects(shareDocuments(runtime,[1],{...platform,canShare:()=>false}),/cannot share/);
  assert.equal(calls,0);
  assert.equal(await shareDocuments(runtime,[1],{...platform,share:()=>Promise.reject(Object.assign(Error('cancelled'),{name:'AbortError'}))}),false);
  assert.throws(()=>documentFiles(runtime,[0]),/valid documents/);
});
