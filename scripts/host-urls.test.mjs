import test from 'node:test';import assert from 'node:assert/strict';
import {webUrl,dialUrl,smsUrl,mailUrl}from'../crates/phone-browser/wwwroot/urls.js';
test('untrusted bookmark URLs cannot become executable or credential-bearing navigation',()=>{
  for(const input of ['javascript:alert(1)','data:text/html,hi','file:///etc/passwd','https://name:password@example.com'])assert.throws(()=>webUrl(input));
  assert.equal(webUrl('example.com/a?q=a b'),'https://example.com/a?q=a%20b');
});
test('composer handoffs encode user text and refuse injected recipients',()=>{
  assert.match(smsUrl('+1 202 555 0101','a&body=b#c'),/body=a%26body%3Db%23c$/);
  assert.throws(()=>dialUrl('123?call=456'));
  assert.throws(()=>mailUrl('a@example.com\r\nBcc:other@example.com','',''));
  assert.equal(mailUrl('a@example.com','hello & goodbye','text#'), 'mailto:a%40example.com?subject=hello%20%26%20goodbye&body=text%23');
});
