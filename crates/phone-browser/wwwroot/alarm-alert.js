// One reusable audio context and one timer for the currently presented alert.
// Scheduling, snooze and dismissal belong to the shared Rust phone session.
let context;
let timer;
let active = false;
let volume = 0;
let vibration = false;
let report = () => {};
let warned = false;
let attached = false;
let foreground = true;
const voices = new Set();

function vibrate(pattern) {
  try { navigator.vibrate?.(pattern); } catch {}
}

function silence() {
  for (const oscillator of voices) { try { oscillator.stop(); } catch {} }
  voices.clear();
  if (vibration) vibrate(0);
}

function pulse() {
  if (!active || !foreground || document.hidden) return;
  if (vibration) vibrate([180,100,180,100,300]);
  if (!volume) return;
  try {
    const Audio = window.AudioContext || window.webkitAudioContext;
    if (!Audio) { if (!warned) report('Alarm sound is unavailable in this browser.'); warned=true; return; }
    context ||= new Audio();
    void context.resume().catch(() => {});
    if (context.state !== 'running') {
      if (!warned) report('Tap the alarm to allow sound in this browser.');
      warned = true;
      return;
    }
    for (let index=0;index<3;index++) {
      const oscillator=context.createOscillator(); const gain=context.createGain();
      const start=context.currentTime+index*.26;
      oscillator.frequency.value=index===1?660:880;
      gain.gain.setValueAtTime(0,start);
      gain.gain.linearRampToValueAtTime(.18*(volume/10),start+.015);
      gain.gain.setValueAtTime(.18*(volume/10),start+.14);
      gain.gain.linearRampToValueAtTime(0,start+.20);
      oscillator.connect(gain).connect(context.destination);
      voices.add(oscillator);
      oscillator.onended=()=>{voices.delete(oscillator);oscillator.disconnect();gain.disconnect();};
      oscillator.start(start);oscillator.stop(start+.21);
    }
  } catch (error) {
    if (!warned) report(`Alarm sound could not play: ${error.message||error}`);
    warned = true;
  }
}

export function setAlarmAlert(next) {
  const nextActive=Boolean(next.active),nextVolume=Math.max(0,Math.min(10,Number(next.volume)||0)),nextVibration=Boolean(next.vibration);
  if(active===nextActive&&volume===nextVolume&&vibration===nextVibration)return;
  silence();active=nextActive;volume=nextVolume;vibration=nextVibration;warned=false;
  clearInterval(timer);timer=undefined;
  if(active){pulse();timer=setInterval(pulse,2500);}
}

export function attachAlarmAudio(notice) {
  report=notice;
  if(attached)return;
  attached=true;
  const resume=()=>{
    const current=context;
    if(active&&foreground&&volume&&current?.state==='suspended'){
      void current.resume().then(()=>{if(context===current)pulse();}).catch(()=>{});
    }
  };
  document.addEventListener('pointerdown',resume,{capture:true,passive:true});
  document.addEventListener('keydown',resume,{capture:true,passive:true});
  document.addEventListener('visibilitychange',()=>{if(document.hidden)silence();else pulse();});
  window.addEventListener('pagehide',()=>{
    foreground=false;clearInterval(timer);timer=undefined;silence();
    const previous=context;context=undefined;
    try { void previous?.close().catch(()=>{}); } catch {}
  });
  window.addEventListener('pageshow',()=>{foreground=true;if(active&&!timer){pulse();timer=setInterval(pulse,2500);}});
}
