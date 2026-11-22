const CACHE='windows-phone-__BUILD_VERSION__';
const FILES=__PRECACHE__;
const INDEX=new URL('index.html',self.registration.scope).href;
self.addEventListener('install',event=>event.waitUntil(caches.open(CACHE)
  .then(cache=>cache.addAll(FILES.map(file=>new Request(new URL(file,self.registration.scope),{cache:'reload'}))))
  .then(()=>self.skipWaiting())));
self.addEventListener('activate',event=>event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(key=>key.startsWith('windows-phone-')&&key!==CACHE).map(key=>caches.delete(key)))).then(()=>self.clients.claim())));
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
