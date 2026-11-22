import assert from "node:assert/strict";

// The cover's native Viewbox must mount actual child controls, and expanding
// the same folder must retain the child tile IDs used by normal app activation.
export async function verifyLiveFolder(ctx) {
  const {page,check,state,snapshot,until,capture,control,clickName,textPresent,isRoute}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true,live_tiles:true,battery_saver:false});
  const find=app=>fixture.tiles.find(tile=>tile.app===app&&!tile.target&&!tile.children.length);
  const people=find("People"),phone=find("Phone");
  assert(people&&phone,"The folder fixture needs the original People and Phone tiles");
  const children=[{...people,size:"Medium"},{...phone,size:"Medium"}];
  const folder={id:fixture.next_id++,app:"People",size:"Wide",label:"Native live folder",
    children:children.map(tile=>tile.app),folder_tiles:children,target:null};
  fixture.tiles=[folder];
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const peopleCell=element=>element.name?.startsWith(`PeopleCell-${people.id}-`);
  try {
    await check("native live-folder cover, expansion and child activation retain tile identity",async()=>{
      await load(fixture);
      await control(element=>element.name===`Tile-${folder.id}`,"native folder cover");
      await until(async()=>{
        const view=await snapshot();
        assert.equal(view.elements.filter(peopleCell).length,6,
          "The folder Viewbox must mount all six native People miniature cells");
        assert(!view.elements.some(element=>element.name===`FolderTiles-${folder.id}`),
          "The initial folder cover must be collapsed");
        return true;
      },"live content mounted inside the folder cover");
      await capture("start-folder-live-cover");

      await clickName(`Tile-${folder.id}`);
      await until(async()=>{
        const view=await snapshot();
        const expanded=view.elements.find(element=>element.name===`FolderTiles-${folder.id}`);
        assert(expanded&&expanded.width>0&&expanded.height>0,"Expanded folder must have a native tile canvas");
        assert.equal(view.elements.filter(element=>peopleCell(element)&&element.y>=expanded.y).length,6,
          "The expanded folder must mount six full-size People cells below the miniature cover");
        for(const child of children){
          assert(view.elements.some(element=>element.name===`Tile-${child.id}`&&element.hit),
            `Expanded child ${child.app} must retain its original ID and accept input`);
        }
        return true;
      },"native live-folder expansion");
      await capture("start-folder-expanded-children");

      await clickName(`Tile-${phone.id}`);
      await isRoute({App:"Phone"});await textPresent("history","folder child opened Phone");
      await clickName("HardwareBack");await isRoute("Start");
      await control(element=>element.name===`Tile-${folder.id}`,"folder retained after returning from its child");
      const retained=(await state()).tiles.find(tile=>tile.id===folder.id);
      assert(retained,"App activation must retain the folder");
      assert.deepEqual(retained.folder_tiles,children,"App activation must preserve the full child tile records");
    });
  } catch(error) {
    await capture("start-folder-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
  }
}
