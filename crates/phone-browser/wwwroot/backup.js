import {allAssets,putAssets,deleteAsset}from'./phone-storage.js';
import {chooseFiles,restoreAssets}from'./media.js';
import {mediaSources,prepareBackup} from './backup-format.js';
function base64(bytes){let text='';const view=new Uint8Array(bytes);for(let i=0;i<view.length;i+=16384)text+=String.fromCharCode(...view.subarray(i,i+16384));return btoa(text);}
export async function exportBackup(runtime){
  const phone=JSON.parse(runtime.exportData());
  const sources=mediaSources(phone);
  const assets=(await allAssets()).filter(asset=>sources.has(asset.key));
  const missing=[...sources].filter(key=>!assets.some(asset=>asset.key===key));
  let total=0;
  if(assets.length>10000)throw new Error('This backup has too many media files.');
  for(const asset of assets){
    const size=asset.bytes.byteLength;
    if(!size)throw new Error('This backup contains an empty media file. Remove it before exporting.');
    if(size>(asset.kind==='image'?16:128)*1024*1024||(total+=size)>192*1024*1024)throw new Error('Backup media exceeds the portable backup limit. Export or remove some large media files first.');
  }
  const payload={format:'ferroui-windows-phone',version:1,exportedAt:new Date().toISOString(),phone,missingAssets:missing,assets:assets.map(({bytes,...asset})=>({...asset,data:base64(bytes)}))};
  const blob=new Blob([JSON.stringify(payload)],{type:'application/json'});
  if(blob.size>256*1024*1024)throw new Error('Backup exceeds the 256 MB portable backup limit. Remove some large media files first.');
  const url=URL.createObjectURL(blob);
  const link=document.createElement('a');link.href=url;link.download=`windows-phone-${new Date().toISOString().slice(0,10)}.json`;link.click();setTimeout(()=>URL.revokeObjectURL(url),30000);
  runtime.hostStatus(missing.length?`Backup exported; ${missing.length} media file(s) were missing from this browser.`:'Backup exported.');
}
export async function importBackup(runtime){
  const [file]=await chooseFiles('.json,application/json');if(!file)return;
  if(file.size>256*1024*1024)throw new Error('Backup exceeds the 256 MB import limit.');
  const payload=JSON.parse(await file.text());
  const {json,assets,missing}=prepareBackup(payload,text=>runtime.validateData(text));
  let committed=false;
  try{
    await putAssets(assets);runtime.importData(json);committed=true;
    await restoreAssets(runtime);
    runtime.hostStatus(missing?`Your backup has been restored; ${missing} media file(s) were absent from the backup.`:'Your backup has been restored.');
  }finally{
    // New immutable keys cannot damage the previous phone's media on failure.
    if(!committed)await Promise.allSettled(assets.map(asset=>deleteAsset(asset.key)));
  }
}
