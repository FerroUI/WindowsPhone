import assert from "node:assert/strict";

// Native inputs and app commands drive real app-owned draft persistence. The
// only email-app action uses an invalid address, so this test never launches a
// mail client or sends a message. Successful handoff is covered by Rust's mock.
export async function verifyOutlookDrafts(ctx) {
  const {page,check,state,snapshot,until,capture,button,centre,control,
    clickName,clickLabel,edit,isRoute,reveal,reload}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true});
  fixture.app_data||={};
  delete fixture.app_data["phone.outlook"];
  const fields={to:"ada@example.com",subject:"Native Outlook draft",body:"First line from the native editor.\n\nSecond paragraph — saved locally."};
  const saved=async()=>(await state()).app_data?.["phone.outlook"];
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const app=async()=>{
    await page.evaluate('phoneApp.openApp("Outlook")');
    await isRoute({App:"Outlook"});
  };
  const scrollMatch=element=>/ScrollViewer$/.test(element.type)&&element.width>220&&element.height>250;
  const field=async(name,value)=>{
    if(!(await snapshot()).elements.some(element=>element.hit&&element.name===name)){
      await page.wheel(...centre(await control(scrollMatch,"Outlook editor scroll viewer")),0,-10000);
      await reveal(element=>element.name===name,name,scrollMatch,"Outlook editor scroll viewer");
    }
    await edit(name,value);
  };
  const drafts=async()=>{await clickLabel("more");await clickLabel("drafts");};
  const closeEditor=async()=>{
    await clickName("HardwareBack");
    await until(async()=>(await snapshot()).elements.every(element=>element.name!=="MailComposeBody"),"Outlook editor closed");
  };
  let id;
  try {
    await load(fixture);
    await check("native Outlook multiline editor saves drafts only after explicit save",async()=>{
      await app();await clickLabel("new");
      for(const [name,value] of [["MailComposeTo",fields.to],["MailComposeSubject",fields.subject],["MailComposeBody",fields.body]]){
        await field(name,value);
      }
      assert.equal(await saved(),undefined,"typing cannot persist a draft");
      await capture("outlook-native-multiline-editor");
      await clickLabel("save draft");
      const library=await until(async()=>{const value=await saved();return value?.items.length===1&&value;},"local Outlook draft saved");
      assert.equal(library.version,1);
      id=library.items[0].id;
      assert.deepEqual(library.items[0].fields,fields);
      const persisted=await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
      assert.deepEqual(persisted.app_data["phone.outlook"],library);
      assert.deepEqual((await state()).mail,fixture.mail,"saving a draft must not fabricate sent or inbox mail");
      await closeEditor();await drafts();
      await control(element=>element.name===`MailDraft-${id}`,"saved Outlook draft row");
      await capture("outlook-local-drafts");
      await clickName(`MailDraft-${id}`);
    });

    await check("Outlook Back cancellation and app switching preserve unsaved text without replacing the saved draft",async()=>{
      const unsaved="Unsaved replacement\nKeep this while switching apps.";
      await field("MailComposeBody",unsaved);
      await clickName("HardwareBack");
      await control(element=>button(element)&&element.automationName==="discard","discard unsaved changes confirmation");
      await capture("outlook-unsaved-back-confirmation");
      await clickLabel("cancel");
      assert((await snapshot()).elements.some(element=>element.name==="MailComposeBody"&&element.text===unsaved));
      assert.deepEqual((await saved()).items[0].fields,fields);
      await clickName("HardwareStart");await isRoute("Start");await app();
      assert((await snapshot()).elements.some(element=>element.name==="MailComposeBody"&&element.text===unsaved),
        "the retained composer must survive app switching");
      await clickName("HardwareBack");await clickLabel("discard");
      await until(async()=>(await snapshot()).elements.every(element=>element.name!=="MailComposeBody"),"unsaved replacement discarded");
      assert.deepEqual((await saved()).items[0].fields,fields);
      await reload();await app();await drafts();await clickName(`MailDraft-${id}`);
      assert((await snapshot()).elements.some(element=>element.name==="MailComposeBody"&&element.text===fields.body),
        "saved multiline draft must survive a complete runtime reload");
      await capture("outlook-restored-draft");
    });

    await check("Outlook rejects an invalid handoff and deletes a local draft only after confirmation",async()=>{
      await field("MailComposeTo","missing email address");
      await clickLabel("open email");
      await until(async()=>(await snapshot()).elements.some(element=>element.text===
        "Enter an email address. Separate multiple addresses with commas."),"invalid recipient validation");
      assert.deepEqual((await saved()).items[0].fields,fields);
      assert.deepEqual((await state()).mail,fixture.mail);
      await clickLabel("delete draft");await clickLabel("cancel");
      assert.equal((await saved()).items.length,1,"canceling deletion preserves the saved draft");
      await clickLabel("delete draft");await clickLabel("delete");
      await until(async()=>(await saved()).items.length===0,"confirmed draft deletion persisted");
      assert.deepEqual((await state()).mail,fixture.mail);
      await capture("outlook-draft-deleted");
    });
  } catch(error) {
    await capture("outlook-drafts-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
  }
}
