#!/usr/bin/env python3
"""Check real shell-owned private data admission, rendered badge/rows and revocation."""
import argparse
import json
import re
import sys
from pathlib import Path

from verify_android_ui import number, rectangle, require, verify as verify_view


def listener_set(text):
    """Normalize Android's read-only colon-separated setting, never rewrite it."""
    text = text.strip()
    if text in ('', 'null'):
        return []
    result = set()
    for component in text.split(':'):
        require(re.fullmatch(r'[A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)+/[A-Za-z0-9_.$]+', component) is not None,
                'Malformed listener component in Android settings')
        package, name = component.split('/')
        if name.startswith('.'):
            name = package + name
        result.add(package + '/' + name)
    return sorted(result)


def listener_transition(before, observed, component, operation):
    """Permit a pending own-component update, never an unrelated access change."""
    original, current = set(before), set(observed)
    require(current - {component} == original - {component},
            'The command changed unrelated listener access')
    if operation == 'grant':
        expected = original | {component}
    elif operation == 'revoke':
        expected = original - {component}
    elif operation == 'restore':
        expected = original
    else:
        raise ValueError('Unknown listener access transition')
    return current == expected


def verify_help(commands, post):
    require(all(re.search(r'(?m)^\s*' + command + r'\s+COMPONENT\s+\[user_id\b', commands)
                for command in ('allow_listener', 'disallow_listener')),
            'This Android image does not advertise scoped listener component/user operands')
    require(re.search(r'(?im)^\s*usage:\s*cmd\s+notification\s+post\s+\[flags\]\s+(?:<tag>|tag)\s+(?:<text>|text)\s*$', post)
            and re.search(r'(?m)^\s*-t\|--title\s+<text>\s*$', post)
            and re.search(r'(?m)^\s*-h\|--help\s*$', post),
            'This Android image does not advertise the expected notification post syntax')


def fully_visible(value, parent):
    x, y, width, height = rectangle(value)
    px, py, pw, ph = rectangle(parent)
    return value.get('fully_visible') is True and x >= px - 1 and y >= py - 1 and x + width <= px + pw + 1 and y + height <= py + ph + 1


def notification_swipe(proof, nonce, after):
    """Choose native input inside the fixture's measured list, outside shell dismissal."""
    verify(proof, nonce, 'notifications-admitted', after)
    center, scroll = proof.get('action_center'), proof.get('notification_scroll')
    require(center is not None and fully_visible(center, proof['inner']),
            'Cannot scroll an absent or clipped Action Center')
    require(isinstance(scroll, dict) and fully_visible(scroll['bounds'], center),
            'The fixture has no single visible notification ScrollViewer')
    x, y, width, height = rectangle(scroll['bounds'])
    viewport = scroll['viewport']
    require(abs(number(viewport['width']) - width) <= 1 and abs(number(viewport['height']) - height) <= 1,
            'The measured list bounds do not match its scrolling viewport')
    targets = scroll['targets']
    require(len(targets) == 2 and {row['text'] for row in targets} == {
        'FerroUI-bootstrap-' + nonce, 'Native-listener-' + nonce},
        'Scrolling requires the exact title and body in the same list')
    rectangles = [rectangle(row['bounds']) for row in targets]
    top = min(row[1] for row in rectangles)
    bottom = max(row[1] + row[3] for row in rectangles)
    require(bottom - top <= height - 16, 'The title and body cannot fit together in the notification viewport')
    # phone-system gestures/controller.rs reserves the final 76 DIP above the
    # 54-DIP navigation bar for closing the overlay. The old bottom-65 swipe
    # entered that region and closed Action Center instead of scrolling its list.
    _, screen_y, _, screen_height = rectangle(proof['screen'])
    safe_top = y + 24
    safe_bottom = min(y + height - 24, screen_y + screen_height - 54 - 76 - 16)
    distance = min(140, safe_bottom - safe_top)
    require(distance >= 48, 'No safe notification scroll range outside the shell close gesture')
    offset = number(scroll['offset']['y'])
    extent = number(scroll['extent']['height'])
    require(0 <= offset <= max(0, extent - height) + 1, 'Invalid native notification scroll offset')
    if bottom > y + height - 8:
        require(offset < extent - height - 1, 'The clipped fixture is already at the end of the native list')
        shift = min(distance, max(48, bottom - (y + height - 8)))
        start, end = safe_bottom, safe_bottom - shift
    elif top < y + 8:
        require(offset > 1, 'The clipped fixture is already at the beginning of the native list')
        shift = min(distance, max(48, y + 8 - top))
        start, end = safe_top, safe_top + shift
    else:
        raise ValueError('Fixture geometry is in range but its rendered labels failed; refusing blind input')
    scale = number(proof['render_scaling'])
    require(scale > 0, 'Missing native render scale')
    return round((x + width / 2) * scale), round(start * scale), round(end * scale)


def verify(proof, nonce, phase, after, sms=None):
    sequence = verify_view(proof, nonce, 'ferroui-phone://home', after)
    roles = {row['role']: row['status'] for row in proof['device_snapshot']['roles']}
    operations = set(proof['device_snapshot']['operations'])
    require(roles.get('dialer') == 'active' and roles.get('sms') == 'active',
            'The shared snapshot did not observe the explicitly selected carrier roles')
    require('readMessages' in operations, 'The shared snapshot has no usable message read authority')
    require(proof['device_message_thread'] is None and not proof['device_message_ids'],
            'Messaging was opened before shell bootstrap was verified')
    if phase == 'start-unread':
        require(sms is not None and sms['nonce'] == nonce, 'Missing matching native SMS proof')
        native = sms['received_sms']
        require(native['matching_rows'] == 1, 'The modem fixture was not persisted exactly once')
        threads = [row for row in proof['device_threads'] if row['id'] == native['thread_id']]
        require(len(threads) == 1 and threads[0]['unread'] == 1 and proof['device_message_unread'] == 1,
                'Start has not admitted the exact native unread conversation')
        require(proof.get('action_center') is None, 'Start badge proof must precede opening Action Center')
        badges = [row for row in proof['messaging_badges']
                  if row['tile_id'] in proof['messaging_tile_ids'] and row['name'] == 'TileBadge-' + str(row['tile_id'])
                  and row['text'] == '1' and fully_visible(row['bounds'], proof['inner'])]
        require(bool(badges), 'The mounted canonical Messaging tile has no visible unread badge')
    elif phase in ('notifications-admitted', 'notifications-visible'):
        require(roles.get('notificationListener') == 'active' and 'readNotifications' in operations,
                'Android has not connected the granted notification listener')
        fixture = proof['device_notification_fixture']
        title, body = 'FerroUI-bootstrap-' + nonce, 'Native-listener-' + nonce
        require(len(fixture) == 1 and fixture[0]['package_id'] == 'com.android.shell'
                and fixture[0]['title'] == title and fixture[0]['body'] == body and fixture[0]['key'],
                'The shared store did not admit the actual unique shell notification')
        require(proof['device_notification_count'] >= 1, 'The fixture is missing from admitted notifications')
        if phase == 'notifications-visible':
            center = proof.get('action_center')
            require(center is not None and fully_visible(center, proof['inner']), 'The real Action Center is not visible')
            labels = {row['text'] for row in proof['notification_fixture_labels'] if fully_visible(row['bounds'], center)}
            require({title, body} <= labels, 'The fixture title and body are not visibly rendered in Action Center')
    elif phase == 'notifications-revoked':
        require(roles.get('notificationListener') != 'active' and 'readNotifications' not in operations,
                'The shared snapshot retained revoked listener authority')
        require(proof['device_notification_count'] == 0 and not proof['device_notification_fixture'],
                'Revocation did not clear admitted notification rows')
        require(proof.get('action_center') is not None and not proof['notification_fixture_labels'],
                'Revoked private text remained rendered in Action Center')
    elif phase == 'start-restored':
        require(proof.get('action_center') is None, 'The real Back input did not close Action Center')
    else:
        raise ValueError('Unknown bootstrap phase')
    return sequence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('observation', type=Path)
    parser.add_argument('nonce')
    parser.add_argument('phase')
    parser.add_argument('after', type=int)
    parser.add_argument('--sms', type=Path)
    args = parser.parse_args()
    try:
        print(verify(json.loads(args.observation.read_text()), args.nonce, args.phase, args.after,
                     json.loads(args.sms.read_text()) if args.sms else None))
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
