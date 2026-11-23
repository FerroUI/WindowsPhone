#!/usr/bin/env python3
"""Contract regressions for actual Telecom notification ownership and teardown."""
import copy
import sys
import unittest

sys.dont_write_bytecode = True
from verify_android_device import verify

NONCE = 'a' * 32


def observation(state='active'):
    return {
        'schema': 1, 'nonce': NONCE, 'sequence': 5,
        'codec': {'result': 'passed', 'assertions': 13, 'rf_requests': 0},
        'snapshot': {
            'roles': [{'role': 'dialer', 'status': 'active'}, {'role': 'sms', 'status': 'active'}],
            'controls': [{} for _ in range(25)],
            'operations': ['readMessages', 'sendMessages', 'sendMultimediaMessages', 'activeCalls', 'callControl'],
        },
        'native_call_visible': True,
        'calls': [{'state': state, 'incoming': True}],
        'call_presentation': {'service_type': 4, 'foreground_notification': True, 'call_style': True, 'error': None},
    }


class CallPresentationContract(unittest.TestCase):
    def test_ringing_and_active_require_actual_type_notification_and_style(self):
        for state in ['ringing', 'active']:
            proof = observation(state)
            self.assertEqual(verify(proof, NONCE, state, 4), 5)
            for field, value in [('service_type', 0), ('service_type', 2), ('foreground_notification', False), ('call_style', False)]:
                invalid = copy.deepcopy(proof)
                invalid['call_presentation'][field] = value
                with self.subTest(state=state, field=field, value=value), self.assertRaisesRegex(ValueError, 'foreground owner'):
                    verify(invalid, NONCE, state, 4)

    def test_rejected_call_notification_is_not_accepted_as_a_visible_call(self):
        proof = observation()
        proof['call_presentation']['error'] = 'Android rejected foreground admission'
        with self.assertRaisesRegex(ValueError, 'Android rejected call-notification'):
            verify(proof, NONCE, 'active', 4)

    def test_final_call_releases_both_system_foreground_type_and_notification(self):
        proof = observation('disconnected')
        with self.assertRaisesRegex(ValueError, 'left a foreground'):
            verify(proof, NONCE, 'ended', 4)
        proof['call_presentation']['service_type'] = 0
        with self.assertRaisesRegex(ValueError, 'left a foreground'):
            verify(proof, NONCE, 'ended', 4)
        proof['call_presentation']['foreground_notification'] = False
        self.assertEqual(verify(proof, NONCE, 'ended', 4), 5)
        proof['calls'] = []
        self.assertEqual(verify(proof, NONCE, 'ended', 4), 5)


if __name__ == '__main__':
    unittest.main()
