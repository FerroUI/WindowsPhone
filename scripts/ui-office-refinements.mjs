import assert from "node:assert/strict";

// Every editing and file-operation assertion follows native pointer/keyboard
// input. The final group separately exercises binary host callbacks without a
// file picker, download, clipboard operation or external service.
export async function verifyOfficeRefinements(ctx) {
  const {page,check,state,snapshot,until,capture,button,centre,control,
    clickName,clickLabel,textPresent,reveal,edit,home,isRoute}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  fixture.settings.lock_timeout=0;
  fixture.settings.reduced_motion=true;
  const names={word:"Native Office letter",excel:"Native Office budget",slides:"Native Office slides",folder:"Native Office folder"};
  const body="Created with native Word controls.\nThis text must survive a complete reload.";
  let wordId,excelId,slidesId,folderId,copiedWordId;
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const app=async label=>{
    await page.evaluate(`phoneApp.openApp(${JSON.stringify(label)})`);
    await isRoute({App:label});
  };
  const scrollMatch=element=>/ScrollViewer$/.test(element.type)&&element.width>220&&element.height>250;
  const revealLabel=async label=>{
    const found=await reveal(element=>button(element)&&element.automationName===label,
      label,scrollMatch,"document list scroll viewer");
    await page.click(...centre(found));
  };
  const field=async(name,value)=>{
    await reveal(element=>element.name===name,name,scrollMatch,"document form scroll viewer");
    await edit(name,value);
  };
  const create=async(kind,name)=>{
    await app("Office");await clickLabel("recent");
    await clickLabel("new");
    await clickLabel({Word:"Word document.docx",Excel:"Excel workbook.xlsx",PowerPoint:"PowerPoint presentation.pptx"}[kind]);
    await edit("FormField0",name);await clickLabel("save");
    const id=await until(async()=>(await state()).documents.find(document=>document.name===name)?.id,`${kind} created through its native form`);
    await isRoute({Document:id});
    return id;
  };
  const reload=async()=>{
    if(ctx.reload){await ctx.reload();return;}
    const origin=await page.evaluate("performance.timeOrigin");
    await page.send("Page.reload",{});
    await until(()=>page.evaluate(`performance.timeOrigin!==${JSON.stringify(origin)}&&!!globalThis.phoneApp?.inspectView`),"reloaded Office runtime",60000);
    await control(element=>element.name==="HardwareStart","reloaded hardware navigation");
  };
  await load(fixture);
  try {
    await check("native Word creation, editing, formatting, reload and pinned activation",async()=>{
      wordId=await create("Word",names.word);
      await clickLabel("edit");await edit("FormField0",body);
      await capture("office-word-native-editor");
      await clickLabel("save");
      await until(async()=>(await state()).documents.find(document=>document.id===wordId)?.content.Word.paragraphs[0].text===body,"Word text persisted");
      await clickLabel("more");await clickLabel("format");await clickLabel("bold");
      await until(async()=>(await state()).documents.find(document=>document.id===wordId)?.content.Word.paragraphs[0].bold===true,"Word paragraph formatting");
      await reload();
      const restored=(await state()).documents.find(document=>document.id===wordId);
      assert.equal(restored.content.Word.paragraphs[0].text,body);
      assert.equal(restored.content.Word.paragraphs[0].bold,true);
      await app("Office");await clickLabel("recent");await revealLabel(`${names.word}.docx`);
      await isRoute({Document:wordId});
      await control(element=>/ListBox$/.test(element.type),"virtualized Word paragraph list");
      await clickLabel("more");await clickLabel("pin to start");await isRoute("Start");
      const pinned=await until(async()=>(await state()).tiles.find(tile=>tile.target?.Document===wordId),"pinned Office destination");
      await page.wheel(...centre(await control(element=>element.name==="StartScroll","Start scroll viewer")),0,-100000);
      const tile=await reveal(element=>element.name===`Tile-${pinned.id}`,"pinned Office tile",element=>element.name==="StartScroll","Start scroll viewer");
      await page.click(...centre(tile));await isRoute({Document:wordId});
      await textPresent(names.word,"pinned Word document title");
      await capture("office-word-persisted-and-pinned");
    });

    await check("native Excel editing calculates a range formula and preserves worksheets",async()=>{
      excelId=await create("Excel",names.excel);
      for(const [address,value] of [["A1","12"],["A2","8"],["B1","=SUM(A1:A2)"]]){
        await clickLabel(`Cell ${address}`);await edit("FormField0",value);await clickLabel("save");
      }
      await until(async()=>(await state()).documents.find(document=>document.id===excelId)?.content.Excel.sheets[0].cells.B1==="=SUM(A1:A2)","saved Excel formula");
      await textPresent("20","computed Excel range sum");
      await clickLabel("add sheet");
      await until(async()=>(await state()).documents.find(document=>document.id===excelId)?.content.Excel.sheets.length===2,"second worksheet created");
      await clickLabel("Sheet1");await textPresent("20","formula retained after worksheet switch");
      await capture("office-excel-formula-and-worksheets");
    });

    await check("native PowerPoint text and speaker notes remain in their slide",async()=>{
      slidesId=await create("PowerPoint",names.slides);
      await clickLabel("edit");await field("FormField0","Native slide title");
      await field("FormField1","First bullet\nSecond bullet");
      await field("FormField2","Speaker notes from the native editor.");
      await capture("office-powerpoint-native-editor");
      await clickLabel("save");
      await until(async()=>{
        const slide=(await state()).documents.find(document=>document.id===slidesId)?.content.PowerPoint.slides[0];
        return slide?.title==="Native slide title"&&slide.body==="First bullet\nSecond bullet"&&slide.notes==="Speaker notes from the native editor.";
      },"PowerPoint title, body and notes saved");
      await clickLabel("add slide");
      await until(async()=>(await state()).documents.find(document=>document.id===slidesId)?.content.PowerPoint.slides.length===2,"second slide created");
      await clickLabel("‹");await textPresent("Native slide title","first slide restored");
      await capture("office-powerpoint-slide-and-notes");
    });

    await check("native Files hierarchy, multi-copy, move and rename preserve document identity",async()=>{
      await app("Files");await clickLabel("phone");await isRoute({FileFolder:null});
      await clickLabel("new folder");await edit("FormField0",names.folder);await clickLabel("save");
      folderId=await until(async()=>(await state()).document_folders.find(folder=>folder.name===names.folder)?.id,"native folder created");
      await clickLabel("select");await revealLabel(`${names.word}.docx`);await revealLabel(`${names.slides}.pptx`);
      await clickLabel("copy");await revealLabel(names.folder);await isRoute({FileFolder:folderId});await clickLabel("paste here");
      await until(async()=>(await state()).documents.filter(document=>document.folder===folderId).length===2,"both selected files copied atomically");
      let data=await state();copiedWordId=data.documents.find(document=>document.folder===folderId&&document.name===names.word)?.id;
      assert(copiedWordId&&copiedWordId!==wordId);
      assert.equal(data.documents.find(document=>document.id===wordId).folder,null);
      await clickLabel("phone");await isRoute({FileFolder:null});
      await clickLabel("select");await revealLabel(`${names.excel}.xlsx`);await clickLabel("move");
      await revealLabel(names.folder);await isRoute({FileFolder:folderId});await clickLabel("paste here");
      await until(async()=>(await state()).documents.find(document=>document.id===excelId)?.folder===folderId,"move retained workbook identity");
      await clickLabel("select");await revealLabel(`${names.word}.docx`);await clickLabel("more");await clickLabel("rename");
      await edit("FormField0","Renamed native letter");await clickLabel("save");
      await until(async()=>(await state()).documents.find(document=>document.id===copiedWordId)?.name==="Renamed native letter","selected copy renamed");
      await clickName("HardwareBack");await isRoute({FileFolder:folderId});
      await capture("files-native-copy-move-rename");
      await revealLabel("Renamed native letter.docx");await isRoute({Document:copiedWordId});
      data=await state();assert.equal(data.documents.find(document=>document.id===copiedWordId).content.Word.paragraphs[0].text,body);
      assert(data.tiles.some(tile=>tile.target?.Document===wordId),"copy/rename must preserve the original pinned target");
      await capture("files-copied-document-reopened");
    });

    await check("Office binary host callbacks roundtrip real packages and reject damaged imports",async()=>{
      const imported=[];
      for(const [id,name,kind] of [[wordId,"Host roundtrip.docx","Word"],[excelId,"Host roundtrip.xlsx","Excel"],[slidesId,"Host roundtrip.pptx","PowerPoint"]]){
        const result=await page.evaluate(`(() => {
          const file=JSON.parse(phoneApp.exportOffice(${id}));
          if(file.bytes[0]!==80||file.bytes[1]!==75)throw new Error('Export is not an OPC ZIP package');
          const imported=phoneApp.importOffice(${JSON.stringify(name)},Uint8Array.from(file.bytes),0);
          return {id:imported,length:file.bytes.length,name:file.name};
        })()`);
        assert(result.length>200&&result.length<1024*1024);
        imported.push(result.id);
        const data=await state();
        assert.deepEqual(data.documents.find(document=>document.id===result.id).content[kind],data.documents.find(document=>document.id===id).content[kind]);
      }
      const before=(await state()).documents.length;
      const rejected=await page.evaluate(`(() => {try{phoneApp.importOffice('Damaged.docx',new Uint8Array([80,75,3,4]),0);return false;}catch{return true;}})()`);
      assert(rejected);assert.equal((await state()).documents.length,before,"damaged import must not mutate the library");
      await app("Office");await clickLabel("recent");await revealLabel("Host roundtrip.docx");await isRoute({Document:imported[0]});
      await capture("office-imported-mobile-copy");
    });
  } catch(error) {
    await capture("office-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
    await home();
  }
}
