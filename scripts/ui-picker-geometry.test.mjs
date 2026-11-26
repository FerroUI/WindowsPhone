import assert from 'node:assert/strict';
import test from 'node:test';
import {assertPickerColumns} from './ui-phone-pickers.mjs';

const expected=['PART_HourSelector','PART_MinuteSelector'];
const fixture=()=>({elements:[
  {name:'PhonePickerSurface',type:'Grid',x:440,y:87,width:400,height:740,projection:null,hitTestVisible:true},
  {name:'PART_HourSelector',type:'DateTimePickerPanel',x:516,y:143,width:124,height:624,hit:true,enabled:true},
  {name:'PART_MinuteSelector',type:'DateTimePickerPanel',x:640,y:143,width:123,height:624,hit:true,enabled:true},
]});

test('separate native picker columns retain distinct selected-cell input targets',()=>{
  assert.equal(assertPickerColumns(fixture(),expected).length,2);
});

test('overlapping columns fail even if keyboard focus or misleading hit state survives',()=>{
  for(const hit of [false,true]){
    const view=fixture();
    Object.assign(view.elements[1],{focused:true,hit});
    view.elements[2].x=view.elements[1].x;
    assert.throws(()=>assertPickerColumns(view,expected),/native input|nonoverlapping/);
  }
});

test('missing date/time fields and geometry outside the phone cannot pass',()=>{
  const missing=fixture();missing.elements.pop();
  assert.throws(()=>assertPickerColumns(missing,expected),/every visible date\/time field/);
  const outside=fixture();outside.elements[2].x=830;
  assert.throws(()=>assertPickerColumns(outside,expected),/inside the picker page/);
});
