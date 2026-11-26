#!/usr/bin/env python3
"""Reject unsafe or incomplete evidence from the real CallLog regression."""
import copy
import os
import pathlib
import subprocess
import sys
import unittest

sys.dont_write_bytecode = True
from verify_android_calls import verify

NONCE = 'b' * 32


def observation():
    def receipt(revision, count):
        value = {'revision': revision, 'count': count}
        if count:
            value['acknowledgement'] = {'count': count, 'token': f'00000000-0000-0000-0000-{revision:012d}'}
        return {'kind': 'device', 'response': {'kind': 'missedCalls', 'value': value}}
    before = [{'id': index, 'timestamp': 1000 + index, 'type': 3, 'new': 1, 'is_read': int(index == 2)} for index in (1, 2, 3)]
    after = copy.deepcopy(before)
    for row in after[:2]:
        row['new'] = 0
    proof = {
        'schema': 1, 'nonce': NONCE, 'kind': 'real_calllog_provider_fixture', 'result': 'passed', 'phase': 'complete', 'rf_requests': 0,
        'started_at': 1000, 'completed_at': 2000,
        'guards': {**{key: True for key in ('debug', 'emulator', 'foreground', 'dialer', 'writable', 'nonce_marker')},
                   'hardware': 'ranchu', 'product': 'sdk_gphone64_x86_64', 'abis': ['x86_64']},
        'inserted_rows': [{'id': row['id'], 'timestamp': row['timestamp'], 'number': f"555014{row['id']}", 'is_read': row['is_read']} for row in before],
        'history_limit': 1, 'history_receipt': {'kind': 'device', 'response': {'kind': 'callHistory', 'value': [{'id': 'call-log:2'}]}},
        'rows_initial': [], 'rows_before_ack': before, 'rows_after_ack': after, 'rows_after_repeat': copy.deepcopy(after),
        'rows_after_cleanup': [], 'repeat_receipt': {'kind': 'error', 'message': 'Selection expired'},
        'cleanup': {'deleted_ids': [1, 2, 3], 'remaining_ids': [], 'errors': []},
    }
    for revision, (name, count) in enumerate((('initial', 0), ('captured', 2), ('before_ack', 3), ('after_ack', 1), ('after_repeat', 1), ('cleanup', 0)), 1):
        proof[f'{name}_receipt'] = receipt(revision, count)
    return proof


class CallLogEvidenceTests(unittest.TestCase):
    def test_real_shell_guard_refuses_existing_or_unowned_emulators_before_device_actions(self):
        script = pathlib.Path(__file__).with_name('smoke-android-calls.sh')
        for changes in ({'emulator_started': '0'}, {'FERROUI_EMU_REUSE': '1'}, {'FERROUI_AVD': 'personal_avd'}):
            environment = {**os.environ, 'emulator_started': '1', 'FERROUI_EMU_REUSE': '0',
                           'FERROUI_AVD': 'phone_ui_contract', 'phone_device_nonce': NONCE, **changes}
            result = subprocess.run(['bash', str(script)], env=environment, capture_output=True, text=True, timeout=5)
            with self.subTest(changes=changes):
                self.assertEqual(result.returncode, 1)
                self.assertNotIn('CallLog', result.stdout)

    def test_accepts_only_the_complete_native_count_flag_receipt_and_cleanup_proof(self):
        self.assertIn('3→1', verify(observation(), NONCE))

    def test_requires_every_native_guard_and_the_current_nonce(self):
        for guard in ('debug', 'emulator', 'foreground', 'dialer', 'writable', 'nonce_marker'):
            value = observation()
            value['guards'][guard] = False
            with self.subTest(guard=guard), self.assertRaises(ValueError): verify(value, NONCE)
        with self.assertRaises(ValueError): verify(observation(), 'a' * 32)

    def test_rejects_preexisting_calls_and_history_page_counts_used_as_global_counts(self):
        for name, wrong in (('initial', 1), ('captured', 1), ('before_ack', 2), ('after_ack', 0)):
            value = observation()
            value[f'{name}_receipt']['response']['value']['count'] = wrong
            with self.subTest(name=name), self.assertRaises(ValueError): verify(value, NONCE)
        value = observation(); value['history_receipt']['response']['value'].append({'id': 'call-log:1'})
        with self.assertRaises(ValueError): verify(value, NONCE)

    def test_rejects_later_call_acknowledgment_and_any_is_read_change(self):
        for index, field, wrong in ((2, 'new', 0), (0, 'is_read', 1), (1, 'is_read', 0), (0, 'timestamp', 999)):
            value = observation(); value['rows_after_ack'][index][field] = wrong
            value['rows_after_repeat'] = copy.deepcopy(value['rows_after_ack'])
            with self.subTest(index=index, field=field), self.assertRaises(ValueError): verify(value, NONCE)

    def test_rejects_repeat_acceptance_stale_receipts_and_unowned_rows(self):
        value = observation(); value['repeat_receipt'] = value['after_ack_receipt']
        with self.assertRaises(ValueError): verify(value, NONCE)
        value = observation(); value['after_ack_receipt']['response']['value']['revision'] = 1
        with self.assertRaises(ValueError): verify(value, NONCE)
        value = observation(); value['rows_before_ack'].append({'id': 99, 'timestamp': 1005, 'type': 3, 'new': 1, 'is_read': 0})
        with self.assertRaises(ValueError): verify(value, NONCE)

    def test_null_unrelated_read_flags_are_preserved_distinctly_from_zero(self):
        value = observation()
        row = {'id': 9, 'timestamp': 900, 'type': 1, 'new': 0, 'is_read': None}
        for name in ('rows_initial', 'rows_before_ack', 'rows_after_ack', 'rows_after_repeat', 'rows_after_cleanup'):
            value[name].append(copy.deepcopy(row))
        verify(value, NONCE)
        value['rows_after_ack'][-1]['is_read'] = 0
        value['rows_after_repeat'] = copy.deepcopy(value['rows_after_ack'])
        with self.assertRaisesRegex(ValueError, 'IS_READ'): verify(value, NONCE)

    def test_rejects_cleanup_failure_or_extra_deletion_and_rf_actions(self):
        for changes in ({'remaining_ids': [3]}, {'deleted_ids': [1, 2]}, {'deleted_ids': [1, 2, 3, 99]}, {'errors': ['write denied']}):
            value = observation(); value['cleanup'].update(changes)
            with self.subTest(changes=changes), self.assertRaises(ValueError): verify(value, NONCE)
        value = observation(); value['rf_requests'] = 1
        with self.assertRaises(ValueError): verify(value, NONCE)


if __name__ == '__main__':
    unittest.main()
