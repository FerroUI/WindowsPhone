import {getAsset,putAssets,deleteAsset}from'./phone-storage.js';
let runtime,report;
let audio=new Audio();let mediaSource='',mediaTitle='';let video=null;let activePlayer=audio;let stream=null;let cameraGeneration=0;
const urls=new Map();
export function attachMedia(api,notice){runtime=api;report=notice;}
export async function restoreAssets(api){
  // The view requests individual photos from IndexedDB as they are needed.
  api.clearImages();
}
export function chooseFiles(accept,multiple=false){
  return new Promise(resolve=>{
    const input=document.createElement('input');input.type='file';input.accept=accept;input.multiple=multiple;input.style.display='none';document.body.append(input);
    const done=files=>{input.remove();resolve(files);};
    input.addEventListener('change',()=>done([...input.files]),{once:true});input.addEventListener('cancel',()=>done([]),{once:true});input.click();
  });
}
async function reducePhoto(file){
  if(file.size>32*1024*1024)throw new Error('Please select an image smaller than 32 MB.');
  const bitmap=await createImageBitmap(file,{imageOrientation:'from-image'});
  try{
    const scale=Math.min(1,1600/Math.max(bitmap.width,bitmap.height));
    const canvas=document.createElement('canvas');canvas.width=Math.max(1,Math.round(bitmap.width*scale));canvas.height=Math.max(1,Math.round(bitmap.height*scale));
    canvas.getContext('2d').drawImage(bitmap,0,0,canvas.width,canvas.height);
    return await new Promise(resolve=>canvas.toBlob(resolve,'image/jpeg',.9));
  }finally{bitmap.close();}
}
async function savePhoto(blob,name){
  const bytes=await blob.arrayBuffer();const key=`media:${crypto.randomUUID()}`;
  await putAssets([{key,name,mime:blob.type||'image/jpeg',kind:'image',bytes}]);
  runtime.photoImported(name,key);return key;
}
export async function importPhotos(){
  for(const file of await chooseFiles('image/*',true)){const photo=await reducePhoto(file);if(photo)await savePhoto(photo,file.name);}
}
export async function importMedia(kind){
  for(const file of await chooseFiles(kind==='video'?'video/*':'audio/*',true)){
    if(file.size>128*1024*1024)throw new Error('Select media files smaller than 128 MB.');
    const key=`media:${crypto.randomUUID()}`;await putAssets([{key,name:file.name,mime:file.type,kind,bytes:await file.arrayBuffer()}]);
    runtime.mediaImported(file.name.replace(/\.[^.]+$/,''),key,kind);
  }
}
export async function objectUrl(key){
  if(urls.has(key))return urls.get(key);
  const asset=await getAsset(key);if(!asset)throw new Error('This media file is missing. Import it again on this device.');
  const url=URL.createObjectURL(new Blob([asset.bytes],{type:asset.mime}));urls.set(key,url);return url;
}
export async function removeMedia(key){await deleteAsset(key);if(urls.has(key)){URL.revokeObjectURL(urls.get(key));urls.delete(key);}if(mediaSource===key){audio.pause();video?.pause();mediaSource='';runtime.mediaChanged('','',false);}}
export async function playMedia(source,surface){
  if(source===mediaSource&&activePlayer.src){await activePlayer.play();return;}
  const asset=await getAsset(source);if(!asset)throw new Error('Import this media file again before playing.');
  audio.pause();video?.pause();
  mediaSource=source;mediaTitle=asset.name.replace(/\.[^.]+$/,'');
  const player=asset.kind==='video'?(video||=document.createElement('video')):audio;
  activePlayer=player;
  if(player===video){player.controls=true;player.playsInline=true;surface.replaceChildren(player);}
  player.src=await objectUrl(source);const settings=JSON.parse(runtime.hostState()).settings;player.volume=settings.media_muted?0:Math.min(1,settings.volume/30);
  player.onended=()=>runtime.mediaChanged(mediaTitle,mediaSource,false);
  player.onerror=()=>{runtime.mediaChanged(mediaTitle,mediaSource,false);report('This browser cannot decode that media format.');};
  player.onplay=()=>runtime.mediaChanged(mediaTitle,mediaSource,true);player.onpause=()=>runtime.mediaChanged(mediaTitle,mediaSource,false);
  try{await player.play();}catch(error){runtime.mediaChanged(mediaTitle,mediaSource,false);throw error;}
  if('mediaSession'in navigator){
    navigator.mediaSession.metadata=new MediaMetadata({title:mediaTitle,artist:'Windows Phone'});
    navigator.mediaSession.setActionHandler('play',()=>player.play());navigator.mediaSession.setActionHandler('pause',()=>player.pause());
  }
}
export async function mediaControl(action){
  const player=activePlayer;
  if(action==='pause'){audio.pause();video?.pause();}
  else if(action==='play'){await activePlayer.play();}
  else if(action==='stop'){audio.pause();audio.currentTime=0;if(video){video.pause();video.currentTime=0;}}
  else if(action==='next'||action==='previous'){const tracks=JSON.parse(runtime.exportData()).tracks;const index=tracks.findIndex(t=>t.source===mediaSource);const next=tracks[(index+(action==='next'?1:tracks.length-1))%tracks.length];if(next)await playMedia(next.source,document.getElementById('media-surface'));}
  else if(action==='rewind')player.currentTime=Math.max(0,player.currentTime-10);
}
export async function startCamera(surface){
  if(!navigator.mediaDevices?.getUserMedia)throw new Error('Camera access is unavailable in this browser.');
  stopCamera();const generation=++cameraGeneration;
  report('Allow camera access to see the viewfinder.');
  const captured=await navigator.mediaDevices.getUserMedia({video:{facingMode:{ideal:'environment'},width:{ideal:1600}},audio:false});
  if(generation!==cameraGeneration){captured.getTracks().forEach(track=>track.stop());return;}
  stream=captured;const preview=document.createElement('video');preview.autoplay=true;preview.playsInline=true;preview.muted=true;preview.srcObject=stream;surface.replaceChildren(preview);await preview.play();
}
export function stopCamera(){cameraGeneration++;stream?.getTracks().forEach(track=>track.stop());stream=null;const surface=document.getElementById('camera-surface');if(surface)surface.replaceChildren();}
export async function capturePhoto(surface){
  if(!stream){await startCamera(surface);return;}
  const preview=surface.querySelector('video');if(!preview?.videoWidth)throw new Error('Wait for the camera preview, then take a photo.');
  const canvas=document.createElement('canvas');canvas.width=preview.videoWidth;canvas.height=preview.videoHeight;canvas.getContext('2d').drawImage(preview,0,0);
  const blob=await new Promise(resolve=>canvas.toBlob(resolve,'image/jpeg',.92));if(!blob)throw new Error('The photo could not be captured.');
  await savePhoto(blob,`Photo ${new Date().toLocaleString()}`);report('Photo saved to camera roll.');
}
export function updateVolume(volume){audio.volume=Math.min(1,Math.max(0,volume/30));if(video)video.volume=audio.volume;}
