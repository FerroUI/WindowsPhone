import {chooseFile} from './sdk-tasks.js';
import {load,save,preserveRecovery,getAsset}from'./phone-storage.js';
import {webUrl,dialUrl,smsUrl,mailUrl}from'./urls.js';
import {attachMedia,importPhotos,importMedia,playMedia,mediaControl,startCamera,capturePhoto,stopCamera,removeMedia,updateVolume}from'./media.js';
import {exportBackup,importBackup}from'./backup.js';
import {importDocuments,exportDocument}from'./documents.js';
import {shareDocuments}from'./document-share.js';
import {attachAlarmAudio,setAlarmAlert}from'./alarm-alert.js';
import {requestNotificationPermission,showSystemNotification} from './system-notifications.js';
import {requestPersistentStorage} from './storage-environment.js';
import {refreshEnvironment} from './environment.js';
import {importContacts,exportContacts,shareContacts,importCalendar,exportCalendar,shareCalendar} from './pim.js';
export {load,save,preserveRecovery,stopCamera};
let runtime;let noticeTimer=0;let surfaceFrame=0;let imageGeneration=0;
const surfaces={};
export const now=()=>Date.now()/1000;
export const utcOffset=()=>-new Date().getTimezoneOffset()*60;
export function cancelImageRequests(){imageGeneration++;}
export function loadImage(key){
  // Queue the read beyond the retained-view render that requested the image.
  const generation=imageGeneration;
  queueMicrotask(async()=>{
    try{const asset=await getAsset(key);if(generation!==imageGeneration)return;if(!asset||asset.kind!=='image')throw new Error('Photo file missing.');runtime.imageReady(key,new Uint8Array(asset.bytes));}
    catch(error){if(generation!==imageGeneration)return;runtime.imageFailed(key);console.info('Photo unavailable:',key,error.message);}
  });
}
export const width=()=>document.getElementById('out').getBoundingClientRect().width;
export const height=()=>document.getElementById('out').getBoundingClientRect().height;
export function attachRuntime(api){runtime=api;attachMedia(api,notice);attachAlarmAudio(notice);for(const kind of ['browser','camera','media']){
  const surface=document.createElement('div');surface.id=`${kind}-surface`;surface.className=`platform-surface ${kind}-surface`;surface.hidden=true;document.getElementById('platform-surfaces').append(surface);surfaces[kind]=surface;
}}
export function notice(message){const element=document.getElementById('host-notice');element.textContent=message;element.hidden=false;clearTimeout(noticeTimer);noticeTimer=setTimeout(()=>element.hidden=true,5000);}
export function storageError(message){notice(`Changes could not be saved. Export a backup. ${message}`);}
export function refreshSurfaces(){
  cancelAnimationFrame(surfaceFrame);surfaceFrame=requestAnimationFrame(()=>{
    if(!runtime)return;
    const settings=JSON.parse(runtime.hostState()).settings;
    applyAppearance(settings);updateVolume(settings.media_muted?0:settings.volume);
    const view=JSON.parse(runtime.inspectView());if(!view)return;
    const host=document.getElementById('out').getBoundingClientRect();
    for(const [kind,name]of[['browser','BrowserViewport'],['camera','CameraViewport'],['media','MediaViewport']]){
      const target=view.elements.find(e=>e.name===name&&e.hit);const surface=surfaces[kind];
      if(!target){surface.hidden=true;if(kind==='camera'&&surface.childElementCount)stopCamera();continue;}
      surface.hidden=surface.childElementCount===0;
      Object.assign(surface.style,{left:`${host.left+target.x}px`,top:`${host.top+target.y}px`,width:`${target.width}px`,height:`${target.height}px`});
    }
  });
}
export function request(json){
  let operation;try{operation=JSON.parse(json);}catch{return;}
  void handle(operation).catch(error=>{notice(error.message||String(error));}).finally(()=>{refreshSurfaces();refreshEnvironment();});
}
async function handle(op){
  switch(op.kind){
      case 'chooseFile': await chooseFile(runtime,op);break;
    case'openUrl':{const link=document.createElement('a');link.href=webUrl(op.url);link.target='_blank';link.rel='noopener noreferrer';link.click();break;}
    case'browse':browse(op.url);break;
    case'clearBrowser':surfaces.browser.replaceChildren();surfaces.browser.hidden=true;break;
    case'dial':location.href=dialUrl(op.number);break;
    case'sms':location.href=smsUrl(op.number,op.body);break;
    case'email':location.href=mailUrl(op.to,op.subject,op.body);break;
    case'importPhotos':await importPhotos();break;
    case'importMusic':await importMedia('audio');break;
    case'importVideo':await importMedia('video');break;
    case'playMedia':await playMedia(op.source,surfaces.media);break;
    case'mediaControl':await mediaControl(op.action);break;
    case'deleteMedia':await removeMedia(op.source);break;
    case'startCamera':await startCamera(surfaces.camera);break;
    case'capturePhoto':await capturePhoto(surfaces.camera);break;
    case'stopCamera':stopCamera();break;
    case'exportBackup':await exportBackup(runtime);break;
    case'importBackup':await importBackup(runtime);break;
    case'importDocuments':await importDocuments(runtime,op.folder,notice);break;
    case'exportDocument':exportDocument(runtime,op.id);break;
    case'shareDocuments':await shareDocuments(runtime,op.ids);break;
    case'importContacts':await importContacts(runtime,notice);break;
    case'exportContacts':exportContacts(runtime,op.ids);break;
    case'shareContacts':await shareContacts(runtime,op.ids);break;
    case'importCalendar':await importCalendar(runtime,notice);break;
    case'exportCalendar':exportCalendar(runtime,op.ids);break;
    case'shareCalendar':await shareCalendar(runtime,op.ids);break;
    case'alarmAlert':setAlarmAlert(op);break;
    case'copyText':if(!navigator.clipboard)throw new Error('Clipboard access is unavailable.');await navigator.clipboard.writeText(op.text);notice('Copied.');break;
    case'locate':{const p=await locate();const url=`https://www.openstreetmap.org/?mlat=${p.coords.latitude}&mlon=${p.coords.longitude}#map=16/${p.coords.latitude}/${p.coords.longitude}`;window.open(url,'_blank','noopener,noreferrer');break;}
    case'fetchWeather':await weather();break;
    case'share':await share(op);break;
    case'notify':await notify(op.title,op.body);break;
    case'requestNotificationPermission':{
      const state=await requestNotificationPermission();
      notice(state==='granted'?'System notifications are allowed.':state==='denied'?'System notifications are blocked. Change this site permission in your browser settings.':'System notifications were not enabled.');
      break;
    }
    case'requestPersistentStorage':notice(await requestPersistentStorage()?'Persistent storage is enabled for this site. Keep backups for recovery.':'Your browser did not grant persistent storage. Your saved phone is still available; keep a backup.');break;
    case'refreshEnvironment':refreshEnvironment();notice('Host information refreshed.');break;
    default:throw new Error(`Unsupported platform request: ${op.kind}`);
  }
}
function browse(value){
  const url=webUrl(value);runtime.browserNavigated(url);const frame=document.createElement('iframe');frame.src=url;frame.title='Web page';frame.referrerPolicy='no-referrer';
  frame.sandbox='allow-scripts allow-forms allow-same-origin allow-popups allow-popups-to-escape-sandbox';
  const tools=document.createElement('div');tools.className='browser-tools';const hint=document.createElement('span');hint.textContent='Some websites require a browser tab.';
  const open=document.createElement('a');open.href=url;open.target='_blank';open.rel='noopener noreferrer';open.textContent='open externally';tools.append(hint,open);surfaces.browser.replaceChildren(frame,tools);frame.addEventListener('load',refreshSurfaces,{once:true});
}
function locate(){if(!JSON.parse(runtime.hostState()).settings.location)return Promise.reject(new Error('Turn on location services in Settings → location, then try again.'));if(!navigator.geolocation)return Promise.reject(new Error('Location is unavailable in this browser.'));return new Promise((resolve,reject)=>navigator.geolocation.getCurrentPosition(resolve,reject,{timeout:15000,maximumAge:300000}));}
async function weather(){
  notice('Choose Allow to get weather at your current location.');const position=await locate();
  const url=new URL('https://api.open-meteo.com/v1/forecast');url.search=new URLSearchParams({latitude:position.coords.latitude,longitude:position.coords.longitude,current:'temperature_2m,weather_code',timezone:'auto'});
  const response=await fetch(url,{signal:AbortSignal.timeout(15000)});if(!response.ok)throw new Error(`Weather service returned ${response.status}.`);
  const data=await response.json();const current=data.current;if(!Number.isFinite(current?.temperature_2m))throw new Error('Weather response was incomplete.');
  const code=current.weather_code;const condition=code===0?'clear sky':code<=3?'partly cloudy':code<=48?'fog':code<=67?'rain':code<=77?'snow':code<=82?'rain showers':code<=86?'snow showers':'thunderstorms';
  runtime.weatherReady('current location',current.temperature_2m,condition);
}
async function share(op){
  const payload={title:op.title,text:op.text};
  if(String(op.url).startsWith('media:')){const asset=await getAsset(op.url);if(asset){const files=[new File([asset.bytes],asset.name,{type:asset.mime})];if(navigator.canShare?.({files}))payload.files=files;}}
  else if(/^https?:/i.test(op.url))payload.url=webUrl(op.url);
  if(navigator.share){try{await navigator.share(payload);}catch(error){if(error.name!=='AbortError')throw error;}}
  else if(navigator.clipboard){await navigator.clipboard.writeText([op.title,op.text,payload.url].filter(Boolean).join('\n'));notice('Copied to clipboard.');}
  else throw new Error('Sharing is unavailable in this browser.');
}
async function notify(title,body){if(!JSON.parse(runtime.hostState()).settings.notifications)return;notice(`${title}: ${body}`);await showSystemNotification(title,body);}
let lastBrightness;
function applyAppearance(settings){
  const brightness=Math.max(.1,Math.min(1,Number(settings.brightness)||.8));
  if(brightness===lastBrightness)return;lastBrightness=brightness;
  const filter=brightness===.8?'none':`brightness(${brightness/.8})`;
  document.getElementById('out').style.filter=filter;
  document.getElementById('platform-surfaces').style.filter=filter;
}
export function serviceTick(){
  if(!runtime)return;
  const data=JSON.parse(runtime.hostState());
  updateVolume(data.settings.media_muted?0:data.settings.volume);applyAppearance(data.settings);
}
