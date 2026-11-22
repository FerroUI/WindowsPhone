import {allAssets,putAssets}from'./phone-storage.js';
import {chooseFiles,restoreAssets}from'./media.js';
function base64(bytes){let text='';const view=new Uint8Array(bytes);for(let i=0;i<view.length;i+=16384)text+=String.fromCharCode(...view.subarray(i,i+16384));return btoa(text);}
export async function exportBackup(runtime){
  const phone=JSON.parse(runtime.exportData());
  const sources=new Set([...phone.photos.map(photo=>photo.source),...phone.tracks.map(track=>track.source),phone.settings.wallpaper].filter(source=>source.startsWith('media:')));
  const assets=(await allAssets()).filter(asset=>sources.has(asset.key));
  const missing=[...sources].filter(key=>!assets.some(asset=>asset.key===key));
  const payload={format:'ferroui-windows-phone',version:1,exportedAt:new Date().toISOString(),phone,missingAssets:missing,assets:assets.map(({bytes,...asset})=>({...asset,data:base64(bytes)}))};
  const blob=new Blob([JSON.stringify(payload)],{type:'application/json'});const url=URL.createObjectURL(blob);
  const link=document.createElement('a');link.href=url;link.download=`windows-phone-${new Date().toISOString().slice(0,10)}.json`;link.click();setTimeout(()=>URL.revokeObjectURL(url),30000);
  runtime.hostStatus(missing.length?`Backup exported; ${missing.length} media file(s) were missing from this browser.`:'Backup exported.');
}
export async function importBackup(runtime){
  const [file]=await chooseFiles('.json,application/json');if(!file)return;
  if(file.size>256*1024*1024)throw new Error('Backup exceeds the 256 MB import limit.');
  const payload=JSON.parse(await file.text());
  if(payload.format==='ferroui-windows-phone'&&payload.version!==1)throw new Error('This backup format is not supported.');
  const data=payload.format==='ferroui-windows-phone'?payload.phone:payload;
  const json=JSON.stringify(data);runtime.validateData(json);
  const assets=[];
  if(payload.assets){
    if(!Array.isArray(payload.assets)||payload.assets.length>10000)throw new Error('Invalid media manifest.');
    for(const item of payload.assets){
      if(typeof item.key!=='string'||!item.key.startsWith('media:')||!['image','audio','video'].includes(item.kind)||typeof item.data!=='string')throw new Error('Invalid media entry.');
      const raw=atob(item.data);assets.push({key:item.key,name:String(item.name||''),mime:String(item.mime||''),kind:item.kind,bytes:Uint8Array.from(raw,c=>c.charCodeAt(0)).buffer});
    }
  }
  await putAssets(assets);runtime.importData(json);await restoreAssets(runtime);runtime.hostStatus('Your backup has been restored.');
}
