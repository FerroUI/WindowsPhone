import {deflateSync} from 'node:zlib';

function crc(bytes){let value=0xffffffff;for(const byte of bytes){value^=byte;for(let bit=0;bit<8;bit++)value=(value>>>1)^((value&1)?0xedb88320:0);}return(value^0xffffffff)>>>0;}
function chunk(name,data){const type=Buffer.from(name),length=Buffer.alloc(4),sum=Buffer.alloc(4);length.writeUInt32BE(data.length);sum.writeUInt32BE(crc(Buffer.concat([type,data])));return Buffer.concat([length,type,data,sum]);}
function tile(){
  const rows=Buffer.alloc(256*(1+256*3));
  for(let y=0;y<256;y++)for(let x=0;x<256;x++){
    const at=y*(1+256*3)+1+x*3;
    const road=x%64<7||y%64<7,river=Math.abs(x-y)<10;
    rows.set(river?[93,157,190]:road?[240,235,222]:[181,197,164],at);
  }
  const header=Buffer.alloc(13);header.writeUInt32BE(256);header.writeUInt32BE(256,4);header[8]=8;header[9]=2;
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
}
/** Install before navigation/gallery traversal. Every scripted map request is
 * fulfilled locally, including requests after page reload. No OSM traffic. */
export async function installMapTileFixture(page){
  const body=tile().toString('base64');
  const observed=[];
  page.on('Fetch.requestPaused',async(event,sessionId)=>{
    if(sessionId)return;
    if(!/^https:\/\/tile\.openstreetmap\.org\/\d+\/\d+\/\d+\.png$/.test(event.request.url)){
      await page.send('Fetch.failRequest',{requestId:event.requestId,errorReason:'BlockedByClient'});return;
    }
    observed.push(event.request.url);
    const result=await page.send('Fetch.fulfillRequest',{requestId:event.requestId,responseCode:200,
      responseHeaders:[{name:'Content-Type',value:'image/png'},{name:'Access-Control-Allow-Origin',value:'*'},
        {name:'Cache-Control',value:'public, max-age=604800'},{name:'X-Phone-Test-Fixture',value:'local-map-tile'}],body});
    if(result?.error)page.errors.push(`Map fixture: ${JSON.stringify(result.error)}`);
  });
  const result=await page.send('Fetch.enable',{patterns:[{urlPattern:'https://tile.openstreetmap.org/*',requestStage:'Request'}]});
  if(result?.error)throw new Error(`Could not install map fixture: ${JSON.stringify(result.error)}`);
  return observed;
}
