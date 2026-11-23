#!/usr/bin/env python3
"""Check real shell-owned private data admission, rendered badge/rows and revocation."""
import argparse
import json
import re
import sys
from pathlib import Path

from verify_android_ui import rectangle, require, verify as verify_view


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


def verify_help(commands, post):
    require(all(command in commands for command in ('allow_listener', 'disallow_listener', 'post')),
            'This Android image does not advertise scoped listener access and notification posting')
    require('--title' in post and 'TAG' in post and 'TEXT' in post,
            'This Android image does not advertise the expected notification post syntax')


def fully_visible(value, parent):
    x, y, width, height = rectangle(value)
    px, py, pw, ph = rectangle(parent)
    return value.get('fully_visible') is True and x >= px - 1 and y >= py - 1 and x + width <= px + pw + 1 and y + height <= py + ph + 1


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
