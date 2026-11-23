import {FerroExports} from './ferroui.js';
import {loadModule} from './ferroui-loader.js';
import {attachRuntime,now,utcOffset,notice,refreshSurfaces,stopCamera,serviceTick} from './host.js';
import {installCanvasSizing} from './canvas-sizing.js';
import {observeEnvironment} from './environment.js';

try{
  installCanvasSizing(document.getElementById('out'));
  const loaded=await loadModule({script:new URL('./phone-browser.js',import.meta.url).href,element:document.getElementById('out')});
  if(!loaded)throw new Error('The WebAssembly runtime could not be loaded.');
  const runtime=await loaded.createRuntime();
  FerroExports.attach(runtime);globalThis.phoneApp=runtime;attachRuntime(runtime);runtime.runMain();
  observeEnvironment(runtime);
  document.addEventListener('keydown',event=>{
    const key={AudioVolumeUp:'volumeUp',AudioVolumeDown:'volumeDown',AudioVolumeMute:'volumeMute'}[event.key];
    if(key){event.preventDefault();runtime.hardwareKey(key);refreshSurfaces();}
  },{capture:true});
  const host=document.getElementById('out');
  let resizeFrame=0;
  const resize=()=>{cancelAnimationFrame(resizeFrame);resizeFrame=requestAnimationFrame(()=>{const bounds=host.getBoundingClientRect();runtime.resize(bounds.width,bounds.height);refreshSurfaces();});};
  new ResizeObserver(resize).observe(host);window.visualViewport?.addEventListener('resize',resize);resize();
  let lastActivity=Date.now();
  for(const type of ['pointerdown','keydown'])document.addEventListener(type,()=>{lastActivity=Date.now();},{capture:true,passive:true});
  for(const type of ['pointerup','keyup'])document.addEventListener(type,()=>{requestAnimationFrame(refreshSurfaces);setTimeout(refreshSurfaces,80);},{capture:true,passive:true});
  const update=()=>{
    if(document.hidden)return;
    runtime.setUtcOffset(utcOffset());
    runtime.tick(now());
    serviceTick();const settings=JSON.parse(runtime.hostState()).settings;
    if(settings.lock_timeout>0&&Date.now()-lastActivity>settings.lock_timeout*1000){runtime.lockPhone();lastActivity=Date.now();refreshSurfaces();}
  };
  setInterval(update,1000);
  document.addEventListener('visibilitychange',()=>{runtime.hostVisibility(!document.hidden);if(document.hidden){runtime.cancelInput();stopCamera();}else{update();resize();}});
  window.addEventListener('blur',()=>runtime.cancelInput());
  window.addEventListener('pagehide',()=>{runtime.hostVisibility(false);runtime.cancelInput();stopCamera();});
  const controller=new AbortController();
  // Pin one navigation sentinel so browser Back has the phone's Back semantics.
  history.replaceState({phone:true},'',location.href);history.pushState({phone:true},'',location.href);
  addEventListener('popstate',()=>{if(!runtime.canGoBack()){history.back();return;}runtime.goBack();history.pushState({phone:true},'',location.href);refreshSurfaces();},{signal:controller.signal});
  if('serviceWorker'in navigator&&!new URLSearchParams(location.search).has('nooffline')){
    navigator.serviceWorker.register(new URL('./app-sw.js',import.meta.url),{scope:'./'}).catch(error=>console.info('Offline installation:',error.message));
  }
}catch(error){
  const status=document.getElementById('loading-status');if(status)status.textContent=`Could not start. ${error.message||error}`;
  document.querySelector('.loading-line')?.remove();console.error(error);notice(error.message||String(error));
}
