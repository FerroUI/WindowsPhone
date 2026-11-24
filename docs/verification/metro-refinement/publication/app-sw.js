const CACHE_PREFIX=`ferroui-phone:${encodeURIComponent(self.registration.scope)}:`;
const CACHE=CACHE_PREFIX+'41d7e3c';
const FILES=["./Selawik-LICENSE.txt","./alarm-alert.js","./app.css","./backup-format.js","./backup.js","./browser-lifecycle.js","./canvas-sizing.js","./document-share.js","./documents.js","./environment.js","./ferroui-loader.js","./ferroui-sw.js","./ferroui.js","./file-picker.js","./host.js","./http-tasks.js","./icon-192.png","./icon-512.png","./icon.svg","./index.html","./main.js","./manifest.webmanifest","./map-tiles.js","./media-session.js","./media.js","./phone-browser.js","./phone-storage.js","./phone_browser.wasm","./pim.js","./platform-bridge.js","./qr-worker.LICENSE.txt","./qr-worker.js","./sdk-tasks.js","./storage-environment.js","./storage.js","./system-notifications.js","./updates.js","./urls.js"];
const INDEX=new URL('index.html',self.registration.scope).href;
self.addEventListener('install',event=>event.waitUntil(caches.open(CACHE)
  .then(cache=>cache.addAll(FILES.map(file=>new Request(new URL(file,self.registration.scope),{cache:'reload'}))))));
self.addEventListener('activate',event=>event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(key=>key.startsWith(CACHE_PREFIX)&&key!==CACHE).map(key=>caches.delete(key)))).then(()=>self.clients.claim())));
self.addEventListener('fetch',event=>{
  const url=new URL(event.request.url);
  if(event.request.method!=='GET'||url.origin!==self.location.origin)return;
  event.respondWith((async()=>{
    const cache=await caches.open(CACHE);
    const navigation=event.request.mode==='navigate';
    const cached=await cache.match(event.request,{ignoreSearch:navigation});
    if(cached)return cached;
    if(navigation&&url.pathname===new URL(self.registration.scope).pathname){
      const index=await cache.match(INDEX);
      if(index)return index;
    }
    return fetch(event.request);
  })());
});
self.addEventListener('notificationclick',event=>{
  event.notification.close();
  // Only app-created notifications resume this app; notification data is never
  // used as an arbitrary URL or executable command.
  if(event.notification.data?.phone!==true)return;
  event.waitUntil((async()=>{
    const scope=new URL(self.registration.scope);
    const windows=await self.clients.matchAll({type:'window',includeUncontrolled:true});
    const client=windows.find(client=>{
      const url=new URL(client.url);
      return url.origin===scope.origin&&url.pathname.startsWith(scope.pathname);
    });
    if(client)return client.focus();
    return self.clients.openWindow(scope.href);
  })());
});
