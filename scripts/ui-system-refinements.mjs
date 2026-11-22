import assert from "node:assert/strict";

// Controlled local fixtures; every workflow mutation below is native pointer or
// keyboard input. No call, SMS, email, clipboard, camera or permission action runs.
export async function verifySystemRefinements(ctx) {
  const {page,check,state,snapshot,until,capture,button,centre,control,
    clickName,clickLabel,textPresent,reveal,edit,home,isRoute}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,ringer_volume:5,volume:19,
    media_muted:false,sounds:true,reduced_motion:false,high_contrast:false,
    lock_detail_app:"Calendar",notifications:true,notifications_on_lock:true});
  fixture.settings.app_notifications={};
  const firstMail=fixture.next_id++,secondMail=fixture.next_id++;
  const contact=fixture.next_id++,firstThread=fixture.next_id++,secondThread=fixture.next_id++;
  const firstMessage=fixture.next_id++,secondMessage=fixture.next_id++;
  const now=Math.floor(Date.now()/1000);
  fixture.contacts.push({id:contact,name:"Smoke Receiver",phone:"+12025550991",email:"",company:"",favorite:false,color:"#1BA1E2"});
  fixture.mail.push(...[
    {id:firstMail,from:"Smoke Alice",subject:"Local mailbox fixture A"},
    {id:secondMail,from:"Smoke Bob",subject:"Local mailbox fixture B"},
  ].map(mail=>({...mail,address:"fixture@example.invalid",body:"Native mailbox selection fixture.",timestamp:now,read:false,flagged:false,folder:"inbox"})));
  fixture.conversations.push(
    {id:firstThread,contact_id:contact,recipient:"+12025550991",unread:true,messages:[
      {id:firstMessage,text:"Nebula fixture body",incoming:true,timestamp:now-30,draft:false},
      {id:secondMessage,text:"Second fixture message",incoming:true,timestamp:now,draft:false},
    ]},
    {id:secondThread,contact_id:null,recipient:"+12025550992",unread:true,messages:[]},
  );
  const mailNotices=[fixture.next_id++,fixture.next_id++],messageNotice=fixture.next_id++;
  fixture.notifications=[
    ...mailNotices.map((id,index)=>({id,app:"Outlook",title:`Smoke mail notice ${index+1}`,body:"Grouped local notification",timestamp:now-index,route:{App:"Outlook"}})),
    {id:messageNotice,app:"Messaging",title:"Smoke message notice",body:"Independent group",timestamp:now-2,route:{App:"Messaging"}},
  ];
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const app=async (label,id=label)=>{
    await page.evaluate(`phoneApp.openApp(${JSON.stringify(label)})`);
    await isRoute({App:id});
  };
  const scrollMatch=element=>/ScrollViewer$/.test(element.type)&&element.width>220&&element.height>160;
  const setting=async name=>{
    await app("Settings");
    await clickLabel("system");
    await page.wheel(...centre(await control(scrollMatch,"Settings scroll viewer")),0,-100000);
    const row=await reveal(element=>button(element)&&element.automationName===name,
      `${name} settings entry`,scrollMatch,"Settings scroll viewer");
    await page.click(...centre(row));
    await isRoute({Setting:name});
  };
  const showLabel=async label=>{
    await page.wheel(...centre(await control(scrollMatch,"Settings scroll viewer")),0,-100000);
    const target=await reveal(element=>button(element)&&element.automationName===label,
      label,scrollMatch,"Settings scroll viewer");
    await page.click(...centre(target));
  };
  const rightClick=async name=>{
    const [x,y]=centre(await control(element=>element.name===name,name));
    await page.mouseMove(x,y);
    await page.send("Input.dispatchMouseEvent",{type:"mousePressed",x,y,button:"right",buttons:2,clickCount:1,pointerType:"mouse"});
    await page.send("Input.dispatchMouseEvent",{type:"mouseReleased",x,y,button:"right",buttons:0,clickCount:1,pointerType:"mouse"});
  };
  await load(fixture);
  try {
    await check("native hardware volume separates ringer and media with stable mute",async()=>{
      await clickLabel("Volume up");
      await until(async()=>(await state()).settings.ringer_volume===6,"ringer increment");
      assert.equal((await state()).settings.volume,19);
      await clickLabel("Expand volume controls");
      await clickLabel("Mute ringer + notifications");
      await until(async()=>(await state()).settings.sounds===false,"ringer muted");
      assert.equal((await state()).settings.media_muted,false);
      await clickLabel("Unmute ringer + notifications");
      await clickLabel("Mute media + apps");
      await until(async()=>(await state()).settings.media_muted===true,"media muted");
      await capture("system-volume-expanded-muted");
      const muted=await state();
      assert.equal(muted.settings.volume,19,"template reconciliation must preserve stored media volume");
      assert.equal(muted.settings.ringer_volume,6);
      assert.equal(muted.settings.sounds,true);
      await clickLabel("Unmute media + apps");
      await clickLabel("Close volume controls");
    });

    await check("native accessibility preferences and lock detail selection persist",async()=>{
      await setting("ease of access");
      await showLabel("Reduce motion, off");
      await until(async()=>(await state()).settings.reduced_motion===true,"reduced motion enabled");
      await showLabel("High contrast, off");
      await until(async()=>(await state()).settings.high_contrast===true,"high contrast enabled");
      await capture("system-high-contrast-settings");
      await showLabel("High contrast, on");
      await showLabel("Reduce motion, on");
      await setting("lock screen");
      await showLabel("Calendar");
      await clickLabel("○ Outlook");
      await until(async()=>(await state()).settings.lock_detail_app==="Outlook","lock detail provider");
      const stored=await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
      assert.equal(stored.settings.lock_detail_app,"Outlook");
      await capture("system-lock-screen-settings");
      await home();
      await clickLabel("Power and lock");
      await control(element=>button(element)&&element.automationName==="Unlock","native lock screen");
      await capture("system-personalized-lock-screen");
      await clickLabel("Unlock");
    });

    await check("native notification policy hides one app and grouped dismissal preserves others",async()=>{
      await setting("notifications+actions");
      await showLabel("Notification settings for Outlook");
      await isRoute({Setting:"notification:Outlook"});
      await showLabel("Show in action center, on");
      await until(async()=>(await state()).settings.app_notifications.Outlook?.action_center===false,"Outlook action-center policy");
      await home();await clickName("StatusBar");
      await control(element=>element.name==="ActionCenter","Action Center");
      await textPresent("Smoke message notice");
      assert(!(await snapshot()).elements.some(element=>element.text==="Smoke mail notice 1"),"disabled app must not be materialized in Action Center");
      assert((await state()).notifications.some(notice=>mailNotices.includes(notice.id)),"hiding notifications does not erase their stored entries");
      await capture("system-filtered-action-center");
      await clickLabel("Close action center");
      await setting("notifications+actions");
      await showLabel("Notification settings for Outlook");
      await showLabel("Show in action center, off");
      await home();await clickName("StatusBar");
      await clickLabel("Dismiss Outlook notifications");
      await until(async()=>!(await state()).notifications.some(notice=>notice.app==="Outlook"),"entire Outlook group dismissed");
      assert((await state()).notifications.some(notice=>notice.id===messageNotice));
      await clickLabel("Close action center");
    });

    await check("native mail selection deletes and restores the selected messages",async()=>{
      await app("Outlook");
      await clickLabel("select");
      await clickLabel("Select email from Smoke Alice");
      await clickLabel("Select email from Smoke Bob");
      await textPresent("2 selected");
      await capture("mail-native-bulk-selection");
      await clickLabel("delete");await clickLabel("save");
      await until(async()=>{
        const data=await state();return [firstMail,secondMail].every(id=>data.mail.find(mail=>mail.id===id)?.folder==="deleted");
      },"both selected messages moved to deleted items");
      await clickLabel("more");await clickLabel("folders");
      await clickLabel("deleted items");
      await clickLabel("select");
      await clickLabel("Select email from Smoke Alice");
      await clickLabel("Select email from Smoke Bob");
      await clickLabel("more");await clickLabel("move to inbox");
      await until(async()=>{
        const data=await state();return [firstMail,secondMail].every(id=>data.mail.find(mail=>mail.id===id)?.folder==="inbox");
      },"selected messages restored to inbox");
      await capture("mail-deleted-folder-after-restore");
    });

    await check("native message search, forwarding draft, contact selection and deletion work",async()=>{
      await app("Messaging");
      await clickLabel("search");await edit("MessageSearch","nebula");
      await clickLabel("Smoke Receiver: Second fixture message");
      await isRoute({Conversation:firstThread});
      await rightClick(`Context-Message-${firstMessage}`);
      await textPresent("message","message context menu");
      await capture("messaging-native-bubble-context");
      await clickLabel("forward");
      await until(async()=>(await snapshot()).elements.some(element=>element.name==="FormField1"&&element.text==="Nebula fixture body"),
        "forward preserves exact selected message text");
      await clickLabel("cancel");
      await rightClick(`Context-Message-${firstMessage}`);
      await clickLabel("delete");await clickLabel("save");
      await until(async()=>!(await state()).conversations.find(thread=>thread.id===firstThread)?.messages.some(message=>message.id===firstMessage),
        "selected individual message removed");
      assert((await state()).conversations.find(thread=>thread.id===firstThread)?.messages.some(message=>message.id===secondMessage));
      await clickName("HardwareBack");await isRoute({App:"Messaging"});
      await clickLabel("new");await edit("RecipientSearch","Smoke Receiver");
      await clickLabel("Smoke Receiver");
      await until(async()=>(await snapshot()).elements.some(element=>element.name==="FormField0"&&element.text==="+12025550991"),
        "contact chooser fills the recipient phone number");
      await capture("messaging-contact-recipient-form");
      await clickLabel("cancel");
      await clickLabel("select");await clickLabel("Select thread with Smoke Receiver");
      const second=await reveal(element=>button(element)&&element.automationName==="Select thread with +12025550992",
        "second selected thread",scrollMatch,"Messaging list scroll viewer");
      await page.click(...centre(second));
      await clickLabel("delete");await clickLabel("save");
      await until(async()=>{
        const data=await state();return !data.conversations.some(thread=>[firstThread,secondThread].includes(thread.id));
      },"both selected threads removed");
    });
  } catch(error) {
    await capture("system-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
  }
}
