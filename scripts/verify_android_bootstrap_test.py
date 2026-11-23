#!/usr/bin/env python3
"""Negative bootstrap contracts: native storage alone cannot prove shared/rendered state."""
import sys
import unittest

sys.dont_write_bytecode = True
from verify_android_bootstrap import listener_set, listener_transition, notification_swipe, verify, verify_help
from verify_android_ui_test import NONCE, mounted

TITLE, BODY = 'FerroUI-bootstrap-' + NONCE, 'Native-listener-' + NONCE
LISTENER = 'org.ferroui.windowsphone.launcher/org.ferroui.phone.PhoneNotificationListener'
# Relevant lines from the unmodified API36 emulator capture in run45273540227.
COMMAND_HELP = '''usage: cmd notification SUBCMD [args]

SUBCMDs:
  allow_listener COMPONENT [user_id (current user if not specified)]
  disallow_listener COMPONENT [user_id (current user if not specified)]
  post [--help | flags] TAG TEXT
'''
POST_HELP = '''usage: cmd notification post [flags] <tag> <text>

flags:
  -h|--help
  -v|--verbose
  -t|--title <text>
  -i|--icon <iconspec>
'''
# Exact unchanged pre/post-grant mirror from run45275444506 attempt2. The system
# had already connected our service; its Settings mirror updated about700ms later.
ORIGINAL_LISTENERS = [
    'com.google.android.apps.nexuslauncher/com.android.launcher3.notification.NotificationListener',
    'com.google.android.as/com.google.android.apps.miphone.aiai.common.notification.service.AiAiNotificationListenerService',
]


def ready():
    proof = mounted()
    proof.update(device_snapshot={
        'roles': [{'role': 'dialer', 'status': 'active'}, {'role': 'sms', 'status': 'active'},
                  {'role': 'notificationListener', 'status': 'available'}],
        'operations': ['readMessages'],
    }, device_message_thread=None, device_message_ids=[], device_threads=[{'id': 'thread:1', 'unread': 1}],
        device_message_unread=1, messaging_tile_ids=[4], messaging_badges=[{
            'tile_id': 4, 'name': 'TileBadge-4', 'text': '1',
            'bounds': {'x': 180, 'y': 210, 'width': 20, 'height': 24, 'fully_visible': True}}],
        device_notification_count=0, device_notification_fixture=[], action_center=None,
        notification_fixture_labels=[])
    return proof


def notified():
    proof = ready()
    proof['device_snapshot']['roles'][2]['status'] = 'active'
    proof['device_snapshot']['operations'].append('readNotifications')
    proof.update(device_notification_count=1, device_notification_fixture=[{
        'key': 'notification:1', 'package_id': 'com.android.shell', 'title': TITLE, 'body': BODY}],
        action_center={'x': 0, 'y': 24, 'width': 360, 'height': 618, 'fully_visible': True},
        notification_fixture_labels=[{'text': text, 'bounds': {
            'x': 18, 'y': 300 + index * 45, 'width': 320, 'height': 40, 'fully_visible': True}}
            for index, text in enumerate((TITLE, BODY))])
    return proof


def scrollable():
    proof = notified()
    # Actual root/safe-area geometry from run45278542129; this run exposed a
    # bottom-65 swipe starting inside the shell's bottom-76 close region.
    proof['safe_area']['top'] = 68
    proof['inner'].update(y=68, height=628)
    proof['screen'].update(y=68, height=628)
    proof['viewport']['height'] = 628
    proof['action_center'].update(y=68, height=574)
    proof['notification_scroll'] = {
        'bounds': {'x': 0, 'y': 300, 'width': 360, 'height': 294, 'fully_visible': True},
        'viewport': {'width': 360, 'height': 294},
        'extent': {'width': 360, 'height': 1200}, 'offset': {'x': 0, 'y': 0},
        'targets': [{'text': TITLE, 'bounds': {'x': 18, 'y': 720, 'width': 324, 'height': 75}},
                    {'text': BODY, 'bounds': {'x': 18, 'y': 800, 'width': 324, 'height': 50}}],
    }
    proof['notification_fixture_labels'] = []
    return proof


SMS = {'nonce': NONCE, 'received_sms': {'matching_rows': 1, 'thread_id': 'thread:1'}}


class BootstrapContract(unittest.TestCase):
    def test_cold_start_requires_shared_native_thread_and_actual_visible_badge(self):
        self.assertEqual(verify(ready(), NONCE, 'start-unread', 4, SMS), 5)
        for field, value in [('device_threads', []), ('device_message_unread', 0), ('messaging_badges', [])]:
            proof = ready(); proof[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify(proof, NONCE, 'start-unread', 4, SMS)

    def test_a_different_native_thread_or_badge_cannot_pass(self):
        for field, value in [('tile_id', 99), ('name', 'TileBadge-99'), ('text', '2')]:
            proof = ready(); proof['messaging_badges'][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify(proof, NONCE, 'start-unread', 4, SMS)
        proof = ready(); proof['device_threads'][0]['id'] = 'thread:2'
        with self.assertRaisesRegex(ValueError, 'exact native'):
            verify(proof, NONCE, 'start-unread', 4, SMS)

    def test_clipped_or_offscreen_badge_is_not_rendered_proof(self):
        for change in [{'fully_visible': False}, {'y': 800}, {'width': 0}]:
            proof = ready(); proof['messaging_badges'][0]['bounds'].update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                verify(proof, NONCE, 'start-unread', 4, SMS)

    def test_opening_messaging_first_cannot_satisfy_cold_start(self):
        for change in [{'device_message_thread': 'thread:1'}, {'device_message_ids': ['sms:1']}]:
            proof = ready(); proof.update(change)
            with self.assertRaisesRegex(ValueError, 'Messaging was opened'):
                verify(proof, NONCE, 'start-unread', 4, SMS)

    def test_store_admission_and_rendered_rows_are_distinct_requirements(self):
        proof = notified()
        self.assertEqual(verify(proof, NONCE, 'notifications-visible', 4), 5)
        proof['notification_fixture_labels'] = []
        self.assertEqual(verify(proof, NONCE, 'notifications-admitted', 4), 5)
        with self.assertRaisesRegex(ValueError, 'visibly rendered'):
            verify(proof, NONCE, 'notifications-visible', 4)

    def test_wrong_source_nonce_or_authority_cannot_supply_the_fixture(self):
        for field, value in [('package_id', 'org.ferroui.windowsphone.launcher'), ('body', 'other'), ('title', 'other')]:
            proof = notified(); proof['device_notification_fixture'][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify(proof, NONCE, 'notifications-admitted', 4)
        proof = notified(); proof['device_snapshot']['operations'].remove('readNotifications')
        with self.assertRaisesRegex(ValueError, 'connected'):
            verify(proof, NONCE, 'notifications-admitted', 4)

    def test_unmounted_or_clipped_notification_text_does_not_pass(self):
        for changed in (None, {'x': 0, 'y': 24, 'width': 0, 'height': 618, 'fully_visible': True}):
            proof = notified(); proof['action_center'] = changed
            with self.assertRaises(ValueError):
                verify(proof, NONCE, 'notifications-visible', 4)
        proof = notified(); proof['notification_fixture_labels'][1]['bounds']['fully_visible'] = False
        with self.assertRaisesRegex(ValueError, 'visibly rendered'):
            verify(proof, NONCE, 'notifications-visible', 4)

    def test_revocation_requires_authority_store_and_visible_text_cleanup(self):
        proof = notified()
        proof['device_snapshot']['roles'][2]['status'] = 'available'
        proof['device_snapshot']['operations'].remove('readNotifications')
        for field, empty in [('device_notification_count', 0), ('device_notification_fixture', []), ('notification_fixture_labels', [])]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify(proof, NONCE, 'notifications-revoked', 4)
            proof[field] = empty
        self.assertEqual(verify(proof, NONCE, 'notifications-revoked', 4), 5)
        proof['device_snapshot']['operations'].append('readNotifications')
        with self.assertRaisesRegex(ValueError, 'revoked listener authority'):
            verify(proof, NONCE, 'notifications-revoked', 4)

    def test_real_back_must_close_the_overlay(self):
        with self.assertRaisesRegex(ValueError, 'Back input'):
            verify(notified(), NONCE, 'start-restored', 4)
        self.assertEqual(verify(ready(), NONCE, 'start-restored', 4), 5)

    def test_listener_snapshot_normalizes_without_losing_other_components(self):
        self.assertEqual(listener_set('null\n'), [])
        self.assertEqual(listener_set('com.example/.Listener:' + LISTENER + ':com.example/com.example.Listener\r\n'),
                         ['com.example/com.example.Listener', LISTENER])
        for value in ['com.example', 'com.example/.Listener:bad', 'com.example/.Listener;grant']:
            with self.assertRaises(ValueError): listener_set(value)

    def test_observed_shell_help_is_required(self):
        verify_help(COMMAND_HELP, POST_HELP)
        verify_help(COMMAND_HELP, POST_HELP.replace('<tag> <text>', 'TAG TEXT'))
        with self.assertRaises(ValueError): verify_help('post', POST_HELP)
        with self.assertRaises(ValueError): verify_help(COMMAND_HELP, 'unknown flags')

    def test_actual_android36_help_requires_every_used_operand_and_option(self):
        for fragment in ['allow_listener COMPONENT', 'disallow_listener COMPONENT', '[user_id']:
            with self.subTest(fragment=fragment), self.assertRaises(ValueError):
                verify_help(COMMAND_HELP.replace(fragment, ''), POST_HELP)
        for fragment in ['<tag>', '<text>', '-t|--title', '-h|--help']:
            with self.subTest(fragment=fragment), self.assertRaises(ValueError):
                verify_help(COMMAND_HELP, POST_HELP.replace(fragment, ''))
        with self.assertRaises(ValueError):
            verify_help(COMMAND_HELP, POST_HELP.replace('cmd notification post', 'cmd different post'))

    def test_captured_listener_mirror_can_be_pending_but_only_exact_access_passes(self):
        before = ORIGINAL_LISTENERS
        granted = before + [LISTENER]
        self.assertFalse(listener_transition(before, before, LISTENER, 'grant'))
        self.assertTrue(listener_transition(before, granted, LISTENER, 'grant'))
        self.assertFalse(listener_transition(before, granted, LISTENER, 'revoke'))
        self.assertTrue(listener_transition(before, before, LISTENER, 'revoke'))
        self.assertFalse(listener_transition(before, granted, LISTENER, 'restore'))
        self.assertTrue(listener_transition(before, before, LISTENER, 'restore'))
        self.assertFalse(listener_transition(granted, before, LISTENER, 'restore'))
        self.assertTrue(listener_transition(granted, granted, LISTENER, 'restore'))

    def test_listener_wait_never_accepts_an_unrelated_grant_or_revocation(self):
        for operation in ['grant', 'revoke', 'restore']:
            for changed in [ORIGINAL_LISTENERS[1:], ORIGINAL_LISTENERS + ['com.example/com.example.Listener']]:
                with self.subTest(operation=operation, changed=changed), self.assertRaisesRegex(ValueError, 'unrelated'):
                    listener_transition(ORIGINAL_LISTENERS, changed, LISTENER, operation)

    def test_notification_swipe_stays_in_its_viewport_and_above_shell_dismissal(self):
        proof = scrollable()
        x, start, end = notification_swipe(proof, NONCE, 4)
        scale = proof['render_scaling']
        threshold = proof['screen']['y'] + proof['screen']['height'] - 54 - 76
        previous = proof['action_center']['y'] + proof['action_center']['height'] - 65
        self.assertGreaterEqual(previous, threshold)
        self.assertLess(start / scale, threshold)
        self.assertEqual(x, 360)
        self.assertTrue(300 < end / scale < start / scale < 594)
        # Choosing input does not claim that hidden labels are already rendered.
        with self.assertRaisesRegex(ValueError, 'visibly rendered'):
            verify(proof, NONCE, 'notifications-visible', 4)

    def test_notification_swipe_can_correct_native_scroll_overshoot(self):
        proof = scrollable()
        proof['notification_scroll']['offset']['y'] = 400
        for row, y in zip(proof['notification_scroll']['targets'], (220, 295)):
            row['bounds']['y'] = y
        _, start, end = notification_swipe(proof, NONCE, 4)
        self.assertTrue(300 < start / 2 < end / 2 < 594)

    def test_notification_swipe_rejects_missing_ambiguous_or_clipped_owner(self):
        for change in (None, {}):
            proof = scrollable(); proof['notification_scroll'] = change
            with self.assertRaises((ValueError, KeyError)):
                notification_swipe(proof, NONCE, 4)
        proof = scrollable(); proof['notification_scroll']['bounds']['fully_visible'] = False
        with self.assertRaisesRegex(ValueError, 'single visible'):
            notification_swipe(proof, NONCE, 4)
        proof = scrollable(); proof['notification_scroll']['targets'][0]['text'] = 'unrelated'
        with self.assertRaisesRegex(ValueError, 'exact title'):
            notification_swipe(proof, NONCE, 4)

    def test_notification_swipe_rejects_no_safe_range_or_unfittable_rows(self):
        proof = scrollable()
        proof['notification_scroll']['bounds'].update(y=520, height=74)
        proof['notification_scroll']['viewport']['height'] = 74
        for row, y in zip(proof['notification_scroll']['targets'], (720, 744)):
            row['bounds'].update(y=y, height=24)
        with self.assertRaisesRegex(ValueError, 'No safe'):
            notification_swipe(proof, NONCE, 4)
        proof = scrollable(); proof['notification_scroll']['targets'][1]['bounds']['y'] = 1100
        with self.assertRaisesRegex(ValueError, 'cannot fit'):
            notification_swipe(proof, NONCE, 4)

    def test_notification_swipe_refuses_blind_input_at_list_boundaries(self):
        proof = scrollable(); proof['notification_scroll']['offset']['y'] = 906
        with self.assertRaisesRegex(ValueError, 'already at the end'):
            notification_swipe(proof, NONCE, 4)
        proof = scrollable()
        for row, y in zip(proof['notification_scroll']['targets'], (350, 430)):
            row['bounds']['y'] = y
        with self.assertRaisesRegex(ValueError, 'refusing blind input'):
            notification_swipe(proof, NONCE, 4)


if __name__ == '__main__':
    unittest.main()
