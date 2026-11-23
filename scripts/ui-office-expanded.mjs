import assert from 'node:assert/strict';

// Native app interactions exercise the composed Rust SDK apps. Fixtures only
// supply pre-existing Office files; every edit below comes from pointer/keyboard input.
export async function verifyOfficeExpansion(ctx) {
  const {page,check,state,snapshot,until,capture,clickLabel,clickName,edit,isRoute,textPresent,reload}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true});
  fixture.alarms=[]; fixture.events=[]; fixture.notes=[];
  delete fixture.app_data['phone.onenote'];
  const word=fixture.next_id++, excel=fixture.next_id++;
  const paragraph={text:'Before editing',bold:false,italic:false,underline:false,heading:0,bullet:false};
  fixture.documents.push(
    {id:word,name:'Undo letter',folder:null,modified:1,import_notice:'',content:{Word:{paragraphs:[paragraph]}}},
    {id:excel,name:'Typed formulas',folder:null,modified:1,import_notice:'',content:{Excel:{sheets:[{name:'Sheet1',cells:{A1:'12',A2:'8'}}]}}},
  );
  const load=async data=>{await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);await isRoute('Start');};
  const app=async name=>{await page.evaluate(`phoneApp.openApp(${JSON.stringify(name)})`);await isRoute({App:name});};
  const doc=async id=>(await state()).documents.find(document=>document.id===id);
  const history=async command=>{await clickLabel('more');await clickLabel(command);};
  await load(fixture);
  try {
    await check('native Word undo and redo restore content and formatting independently',async()=>{
      await app('Office');await clickLabel('recent');await clickLabel('Undo letter.docx');await isRoute({Document:word});
      await clickLabel('edit');await edit('FormField0','After editing');await clickLabel('save');
      await until(async()=>(await doc(word)).content.Word.paragraphs[0].text==='After editing','saved Word edit');
      await history('undo');assert.deepEqual((await doc(word)).content.Word.paragraphs[0],paragraph);
      await history('redo');assert.equal((await doc(word)).content.Word.paragraphs[0].text,'After editing');
      await clickLabel('more');await clickLabel('format');await clickLabel('bold');
      await until(async()=>(await doc(word)).content.Word.paragraphs[0].bold===true,'saved Word bold');
      await history('undo');
      assert.equal((await doc(word)).content.Word.paragraphs[0].text,'After editing');
      assert.equal((await doc(word)).content.Word.paragraphs[0].bold,false);
      await capture('office-word-content-history');
    });
    await check('native Excel evaluates typed conditions and undoes worksheet edits',async()=>{
      await app('Office');await clickLabel('recent');await clickLabel('Typed formulas.xlsx');await isRoute({Document:excel});
      await clickLabel('Cell B1');await edit('FormField0','=IF(SUM(A1:A2)>15,UPPER("ready"),"wait")');await clickLabel('save');
      await textPresent('READY','typed IF, comparison and text result');
      await history('undo');assert(!Object.hasOwn((await doc(excel)).content.Excel.sheets[0].cells,'B1'));
      await history('redo');await textPresent('READY','redone typed formula result');
      await clickLabel('add sheet');await until(async()=>(await doc(excel)).content.Excel.sheets.length===2,'worksheet added');
      await history('undo');assert.equal((await doc(excel)).content.Excel.sheets.length,1);
      await history('redo');assert.equal((await doc(excel)).content.Excel.sheets.length,2);
      await clickLabel('Sheet1');await capture('office-excel-typed-formulas-and-history');
    });
    await check('native OneNote notebooks, sections, checklists and page moves persist',async()=>{
      await app('OneNote');await clickLabel('notebooks');await clickLabel('new notebook');await edit('FormField0','Work notes');await clickLabel('save');
      await textPresent('Work notes','created notebook');
      await clickLabel('new section');await edit('FormField0','Tasks');await clickLabel('save');
      await textPresent('Tasks','created section');
      await clickLabel('new');await edit('FormField0','Shopping checklist');await edit('FormField1','- [ ] buy milk\n- [x] return book');await clickLabel('save');
      const id=await until(async()=>(await state()).notes.find(note=>note.title==='Shopping checklist')?.id,'created page');
      await isRoute({Note:id});
      await clickLabel('to-do: buy milk');
      await until(async()=>(await state()).notes.find(note=>note.id===id)?.body==='- [x] buy milk\n- [x] return book','checkbox persisted in note');
      await textPresent('2 of 2 complete','live checklist progress');
      await clickLabel('more');await clickLabel('add to-do');await edit('FormField0','charge phone');await clickLabel('add');
      await textPresent('2 of 3 complete','new task updates progress');
      await clickLabel('more');await clickLabel('move page');await clickLabel('Quick Notes');
      const metadata=(await state()).app_data['phone.onenote'];
      assert(!Object.hasOwn(metadata.pages,String(id)),'Quick Notes uses the compatible default location');
      await capture('onenote-notebooks-checklist-and-move');
      await reload();
      const restored=await state();
      assert(restored.notes.find(note=>note.id===id)?.body.endsWith('- [ ] charge phone'));
      assert(restored.app_data['phone.onenote'].notebooks.some(notebook=>notebook.name==='Work notes'&&notebook.sections.some(section=>section.name==='Tasks')));
      await app('OneNote');await clickLabel('Shopping checklist');await isRoute({Note:id});
      await textPresent('Personal / Quick Notes','moved page restored');
      await capture('onenote-restored-section-and-checklist');
    });
  } catch(error) {
    await capture('office-expansion-failure').catch(()=>{});
    throw error;
  } finally {await load(original);}
}
