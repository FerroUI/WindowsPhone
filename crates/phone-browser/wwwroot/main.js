import {FerroExports} from './ferroui.js';
import {loadModule} from './ferroui-loader.js';
import {attachRuntime,disposeHost,now,utcOffset,notice,refreshSurfaces,stopCamera,serviceTick} from './host.js';
import {installCanvasSizing} from './canvas-sizing.js';
import {observeEnvironment} from './environment.js';
import {installUpdates} from './updates.js';
import {attachPlatformRuntime} from './platform-bridge.js';
import {bindRuntimeLifecycle,activationFromUrl} from './browser-lifecycle.js';

let disposeStarted=()=>{};
try{
  const host=document.getElementById('out');
  const restoreSizing=installCanvasSizing(host);
  disposeStarted=()=>restoreSizing?.();
  const loaded=await loadModule({script:new URL('./phone-browser.js',import.meta.url).href,element:host});
  if(!loaded)throw new Error('The WebAssembly runtime could not be loaded.');
  const runtime=await loaded.createRuntime();
  const detachPlatform=attachPlatformRuntime(runtime);
  FerroExports.attach(runtime);globalThis.phoneApp=runtime;attachRuntime(runtime);
  disposeStarted=()=>{runtime.closeHost();detachPlatform();disposeHost();restoreSizing?.();};
  runtime.runMain();
  const stopEnvironment=observeEnvironment(runtime);
  const controller=new AbortController();
  const listen=(target,type,callback,options={})=>target?.addEventListener(type,callback,{...options,signal:controller.signal});
  listen(document,'keydown',event=>{
    const key={AudioVolumeUp:'volumeUp',AudioVolumeDown:'volumeDown',AudioVolumeMute:'volumeMute'}[event.key];
    if(key){event.preventDefault();runtime.hardwareKey(key);refreshSurfaces();}
  },{capture:true});
  let resizeFrame=0;
  const resize=()=>{cancelAnimationFrame(resizeFrame);resizeFrame=requestAnimationFrame(()=>{const bounds=host.getBoundingClientRect();runtime.resize(bounds.width,bounds.height);refreshSurfaces();});};
  const observer=new ResizeObserver(resize);observer.observe(host);
  listen(window.visualViewport,'resize',resize);resize();
  let lastActivity=Date.now();
  for(const type of ['pointerdown','keydown'])listen(document,type,()=>{lastActivity=Date.now();},{capture:true,passive:true});
  for(const type of ['pointerup','keyup'])listen(document,type,()=>{requestAnimationFrame(refreshSurfaces);setTimeout(refreshSurfaces,80);},{capture:true,passive:true});
  const update=()=>{
    if(document.hidden)return;
    runtime.setUtcOffset(utcOffset());runtime.tick(now());serviceTick();
    const settings=JSON.parse(runtime.hostState()).settings;
    if(settings?.lock_timeout>0&&Date.now()-lastActivity>settings.lock_timeout*1000){runtime.lockPhone();lastActivity=Date.now();refreshSurfaces();}
  };
  const timer=setInterval(update,1000);
  const activate=()=>{
    try{const uri=activationFromUrl(location.href);if(uri)runtime.activateUri(uri);}
    catch(error){notice(error.message||String(error));}
  };
  listen(window,'hashchange',activate);activate();
  // Embedded surfaces leave navigation history to their owning webpage.
  if(JSON.parse(runtime.platformState()).mode==='app'){
    history.replaceState({phone:true},'',location.href);history.pushState({phone:true},'',location.href);
    listen(window,'popstate',()=>{if(!runtime.canGoBack()){history.back();return;}runtime.goBack();history.pushState({phone:true},'',location.href);refreshSurfaces();});
  }
  let embeddingControls;
  const close=bindRuntimeLifecycle(runtime,{
    onSuspend:stopCamera,
    onResume:()=>{lastActivity=Date.now();update();resize();},
    onClose:()=>{
      controller.abort();observer.disconnect();clearInterval(timer);cancelAnimationFrame(resizeFrame);
      stopEnvironment();detachPlatform();disposeHost();restoreSizing?.();
      if(globalThis.phoneApp===runtime)delete globalThis.phoneApp;
      if(globalThis.phoneHost===embeddingControls)delete globalThis.phoneHost;
      disposeStarted=()=>{};
    },
  });
  disposeStarted=close;
  // The embedding owner can close or activate the same mounted runtime without
  // imitating browser input or taking over its surrounding page's history.
  embeddingControls=Object.freeze({close,activate:uri=>runtime.activateUri(uri),request:request=>runtime.requestPlatform(JSON.stringify(request))});
  globalThis.phoneHost=embeddingControls;
  window.dispatchEvent(new CustomEvent('phonehostready',{detail:embeddingControls}));
  if('serviceWorker'in navigator&&!new URLSearchParams(location.search).has('nooffline')){
    installUpdates(notice).catch(error=>console.info('Offline installation:',error.message));
  }
}catch(error){
  try{disposeStarted();}catch{}
  const status=document.getElementById('loading-status');if(status)status.textContent=`Could not start. ${error.message||error}`;
  document.querySelector('.loading-line')?.remove();console.error(error);notice(error.message||String(error));
}
