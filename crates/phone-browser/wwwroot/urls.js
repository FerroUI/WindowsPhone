export function webUrl(value){
  const text=String(value||'').trim();if(!text)throw new Error('Enter an address.');
  const url=new URL(/^[a-z][a-z\d+.-]*:/i.test(text)?text:`https://${text}`);
  if(!['https:','http:'].includes(url.protocol)||url.username||url.password)throw new Error('Use a normal http or https address.');
  return url.href;
}
export function dialUrl(number){
  const value=String(number).trim();
  if(!/^[+\d ()*#.-]{1,60}$/.test(value))throw new Error('Enter a valid phone number.');
  return `tel:${encodeURIComponent(value.replaceAll(' ',''))}`;
}
export function smsUrl(number,body){
  dialUrl(number);return `sms:${encodeURIComponent(String(number).replaceAll(' ',''))}?body=${encodeURIComponent(body)}`;
}
export function mailUrl(to,subject,body){
  if(/[\r\n]/.test(to)||String(to).length>512)throw new Error('Enter a valid email recipient.');
  return `mailto:${encodeURIComponent(to)}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
}
