import {readFile,writeFile,readdir} from 'node:fs/promises';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const root=path.resolve(process.argv[2]);
const version=execFileSync('git',['rev-parse','--short','HEAD'],{encoding:'utf8'}).trim();
const files=[];
async function scan(dir){for(const item of await readdir(dir,{withFileTypes:true})){const full=path.join(dir,item.name);if(item.isDirectory())await scan(full);else files.push('./'+path.relative(root,full).replaceAll('\\','/'));}}
await scan(root);
for(const file of ['index.html','app-sw.js']){const target=path.join(root,file);let text=await readFile(target,'utf8');text=text.replaceAll('__BUILD_VERSION__',version).replace('__PRECACHE__',JSON.stringify(files.filter(name=>!name.endsWith('.map')&&!name.endsWith('app-sw.js'))));await writeFile(target,text);}
await writeFile(path.join(root,'.nojekyll'),'');
console.log(`Prepared Windows Phone ${version}: ${files.length} files.`);
