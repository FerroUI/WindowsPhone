const KEY='ferroui.windows-phone.v1';
let seen;
let protectedStorage=false;
export function load(){seen=localStorage.getItem(KEY)||'';return seen;}
export function save(text){
  if(protectedStorage)throw new Error('A recovery copy needs your attention. Export a backup before continuing.');
  const current=localStorage.getItem(KEY)||'';
  if(seen!==undefined&&current!==seen)throw new Error('Your phone changed in another tab. Reload before editing to protect those changes.');
  localStorage.setItem(KEY,text);seen=text;
}
export function preserveRecovery(text){
  try{localStorage.setItem(`${KEY}.recovery.${Date.now()}`,text);}catch{protectedStorage=true;}
}
let database;
export function db(){
  if(!database)database=new Promise((resolve,reject)=>{
    const request=indexedDB.open('ferroui-windows-phone-assets',1);
    request.onupgradeneeded=()=>request.result.createObjectStore('assets',{keyPath:'key'});
    request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);
  });
  return database;
}
export async function allAssets(){const connection=await db();return new Promise((resolve,reject)=>{const request=connection.transaction('assets').objectStore('assets').getAll();request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);});}
export async function getAsset(key){const connection=await db();return new Promise((resolve,reject)=>{const request=connection.transaction('assets').objectStore('assets').get(key);request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);});}
export async function putAssets(assets){const connection=await db();await new Promise((resolve,reject)=>{const transaction=connection.transaction('assets','readwrite');for(const asset of assets)transaction.objectStore('assets').put(asset);transaction.oncomplete=resolve;transaction.onerror=()=>reject(transaction.error);transaction.onabort=()=>reject(transaction.error);});}
export async function deleteAsset(key){const connection=await db();await new Promise((resolve,reject)=>{const transaction=connection.transaction('assets','readwrite');transaction.objectStore('assets').delete(key);transaction.oncomplete=resolve;transaction.onerror=()=>reject(transaction.error);});}
