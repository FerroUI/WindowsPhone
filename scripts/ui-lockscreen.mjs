import assert from "node:assert/strict";

// Reproduce automatic locking through the production host callback while an
// unsaved native app form is active. No document is created by this workflow.
export async function verifyLockscreenDraft(ctx) {
  const {page,check,state,snapshot,until,capture,button,control,clickLabel,edit,isRoute}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true,high_contrast:false});
  const draft="Unsaved Office draft survives locking";
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  await load(fixture);
  try {
    await check("lock screen covers an open Office form and preserves its unsaved draft",async()=>{
      await page.evaluate('phoneApp.openApp("Office")');
      await isRoute({App:"Office"});
      await clickLabel("recent");await clickLabel("new");
      await clickLabel("Word document.docx");
      await edit("FormField0",draft);
      const before=await state();

      await page.evaluate("phoneApp.lockPhone()");
      await control(element=>button(element)&&element.automationName==="Unlock","lock screen Unlock button");
      await until(async()=>{
        const view=await snapshot();
        const lock=view.elements.find(element=>element.name==="LockScreen");
        const screen=view.elements.find(element=>element.name==="PhoneScreen");
        assert(lock&&screen,"The native lock screen must be mounted");
        for(const property of ["x","y","width","height"]){
          assert.equal(lock[property],screen[property],`Lock screen must cover the phone ${property}`);
        }
        assert(!view.elements.some(element=>element.hit&&(
          element.name?.startsWith("FormField")||
          (button(element)&&["save","cancel"].includes(element.automationName))
        )),"The form and its commands must not remain interactive above the lock screen");
        return true;
      },"lock screen covers the open app dialog");
      await capture("system-lock-covers-office-draft");

      await clickLabel("Unlock");
      await control(element=>element.name==="FormField0"&&element.text===draft,
        "exact unsaved Office name restored after unlocking");
      assert(!(await snapshot()).elements.some(element=>element.name==="LockScreen"),
        "Unlock must remove the lock screen");
      await capture("system-unlock-restores-office-draft");
      await clickLabel("cancel");
      await until(async()=>!(await snapshot()).elements.some(element=>element.hit&&element.name==="FormField0"),
        "cancel dismissed the restored form");
      const after=await state();
      assert.deepEqual(after.documents,before.documents,"Lock/unlock must not commit the unsaved document");
      assert.equal(after.next_id,before.next_id,"An unsaved draft must not consume a document identity");
    });
  } catch(error) {
    await capture("system-lock-draft-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
  }
}
