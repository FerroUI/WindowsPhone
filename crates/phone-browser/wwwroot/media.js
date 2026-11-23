import {getAsset,putAssets,deleteAsset}from'./phone-storage.js';
import {createMediaSession} from './media-session.js';
import {chooseFiles} from './file-picker.js';
import {publicHttpsUrl} from './http-tasks.js';
export {chooseFiles} from './file-picker.js';
let runtime,report;
let audio=new Audio();let mediaSource='',mediaTitle='';let video=null;let activePlayer=audio;let stream=null;let cameraGeneration=0;
let playbackGeneration=0,playerGeneration=0,mediaArtist='',session;
const urls=new Map();
export function attachMedia(api,notice){runtime=api;report=notice;session?.dispose();session=createMediaSession({session:navigator.mediaSession,Metadata:globalThis.MediaMetadata,control:mediaControl,report:notice});}
export function disposeMedia(){
  playbackGeneration++;playerGeneration++;stopCamera();session?.dispose();session=undefined;
  for(const player of [audio,video].filter(Boolean)){
    player.onended=player.onerror=player.onplay=player.onpause=player.ontimeupdate=player.ondurationchange=null;
    player.pause();player.removeAttribute('src');player.load();
  }
  for(const url of urls.values())URL.revokeObjectURL(url);urls.clear();
  mediaSource='';mediaTitle='';mediaArtist='';runtime=undefined;report=()=>{};
}
export async function restoreAssets(api){
  // A restored phone must not keep the previous document's camera, playback,
  // object URLs or OS lock-screen metadata alive.
  playbackGeneration++;playerGeneration++;session?.clear();audio.pause();video?.pause();
  for(const player of [audio,video].filter(Boolean)){player.removeAttribute('src');player.load();}
  for(const url of urls.values())URL.revokeObjectURL(url);urls.clear();
  mediaSource='';mediaTitle='';mediaArtist='';api.mediaChanged('','',false);stopCamera();
  // The view requests individual photos from IndexedDB as they are needed.
  api.clearImages();
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
    if(!file.size)throw new Error('Select a media file that is not empty.');
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
export async function removeMedia(key){
  await deleteAsset(key);
  if(mediaSource===key){playbackGeneration++;playerGeneration++;session?.clear();audio.pause();video?.pause();activePlayer.removeAttribute('src');activePlayer.load();mediaSource='';mediaTitle='';runtime.mediaChanged('','',false);}
  if(urls.has(key)){URL.revokeObjectURL(urls.get(key));urls.delete(key);}
}
function bindPlaybackEvents(player){
  const generation=++playerGeneration;
  const publish=playing=>{if(generation===playerGeneration)runtime.mediaChanged(mediaTitle,mediaSource,playing);};
  player.onended=()=>{if(generation!==playerGeneration)return;runtime.mediaProgress?.(Number.isFinite(player.currentTime)?player.currentTime:0,Number.isFinite(player.duration)?player.duration:0);runtime.mediaCommand?.('ended');publish(false);};
  player.onerror=()=>{if(generation===playerGeneration){publish(false);report('This browser cannot decode that media format.');}};
  player.onplay=()=>publish(true);player.onpause=()=>publish(false);
  player.ontimeupdate=()=>{if(generation===playerGeneration)runtime.mediaProgress?.(Number.isFinite(player.currentTime)?player.currentTime:0,Number.isFinite(player.duration)?player.duration:0);};
  player.ondurationchange=player.ontimeupdate;
  return publish;
}
export async function playMedia(source,surface){
  const generation=++playbackGeneration;
  if(source===mediaSource&&activePlayer.src){
    const publish=bindPlaybackEvents(activePlayer);session?.attach(activePlayer,{title:mediaTitle,artist:mediaArtist});
    try{await activePlayer.play();}catch(error){if(generation===playbackGeneration){publish(false);throw error;}}
    return;
  }
  let asset,url;
  try{
    if(/^https:/i.test(source)){
      url=publicHttpsUrl(source);
      const track=JSON.parse(runtime.exportData()).tracks.find(track=>track.source===source);
      if(!track)throw new Error('This streaming item is no longer in your library.');
      asset={kind:track.media_kind==='video'?'video':'audio',name:track.title};
    }else asset=await getAsset(source);
    if(generation!==playbackGeneration)return;
    if(!asset||!['audio','video'].includes(asset.kind))throw new Error('Import this media file again before playing.');
    if(!url)url=await objectUrl(source);
  }catch(error){if(generation===playbackGeneration)throw error;return;}
  if(generation!==playbackGeneration)return;
  playerGeneration++;session?.clear();audio.pause();video?.pause();
  mediaSource=source;mediaTitle=asset.name.replace(/\.[^.]+$/,'');
  const track=JSON.parse(runtime.exportData()).tracks.find(track=>track.source===source);
  mediaTitle=track?.title||mediaTitle;mediaArtist=track?.artist||'';
  const player=asset.kind==='video'?(video||=document.createElement('video')):audio;
  activePlayer=player;
  if(player===video){player.controls=true;player.playsInline=true;surface.replaceChildren(player);}
  else surface?.replaceChildren();
  player.src=url;const settings=JSON.parse(runtime.hostState()).settings;player.volume=settings.media_muted?0:Math.min(1,settings.volume/30);
  const publish=bindPlaybackEvents(player);
  session?.attach(player,{title:mediaTitle,artist:mediaArtist});
  try{await player.play();}catch(error){if(generation===playbackGeneration){publish(false);throw error;}}
}
export async function mediaControl(action){
  const player=activePlayer;
  if(action==='pause'){playbackGeneration++;audio.pause();video?.pause();}
  else if(action==='play'){if(mediaSource)await playMedia(mediaSource,document.getElementById('media-surface'));}
  else if(action==='stop'){playbackGeneration++;playerGeneration++;audio.pause();audio.currentTime=0;if(video){video.pause();video.currentTime=0;}session?.clear();runtime.mediaChanged(mediaTitle,mediaSource,false);}
  else if(action==='next'||action==='previous'){
    if(runtime.mediaCommand?.(action))return;
    const kind=activePlayer===video?'video':'audio';
    const tracks=JSON.parse(runtime.exportData()).tracks.filter(track=>(track.media_kind||'audio')===kind);
    const index=tracks.findIndex(track=>track.source===mediaSource);
    const next=index<0?(action==='next'?tracks[0]:tracks.at(-1)):tracks[(index+(action==='next'?1:tracks.length-1))%tracks.length];
    if(next)await playMedia(next.source,document.getElementById('media-surface'));
  }
  else if(action==='restart'){player.currentTime=0;if(mediaSource)await playMedia(mediaSource,document.getElementById('media-surface'));}
  else if(action.startsWith('seek:')){const seconds=Number(action.slice(5));if(!Number.isFinite(seconds)||seconds<0||seconds>31536000)throw new Error('Invalid playback position.');player.currentTime=Number.isFinite(player.duration)?Math.min(seconds,player.duration):seconds;}
  else if(action==='forward')player.currentTime=Math.max(0,Math.min(Number.isFinite(player.duration)?player.duration:Infinity,player.currentTime+10));
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
  await savePhoto(blob,`Photo ${new Date().toISOString().replaceAll(':','-')}.jpg`);report('Photo saved to camera roll.');
}
export function updateVolume(volume){audio.volume=Math.min(1,Math.max(0,volume/30));if(video)video.volume=audio.volume;}
