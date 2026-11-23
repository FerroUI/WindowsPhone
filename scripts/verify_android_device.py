#!/usr/bin/env python3
"""Strict observations from the isolated debug AVD; no simulated provider/route outcome."""
import argparse
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify(proof, nonce, phase, after):
    require(proof['schema'] == 1 and proof['nonce'] == nonce and len(nonce) == 32, 'Foreign device observation')
    require(type(proof['sequence']) is int and proof['sequence'] > after, 'Stale device observation')
    require(not proof.get('error'), proof.get('error', 'Native observation failed'))
    require(proof['codec']['result'] == 'passed' and proof['codec']['assertions'] == 13 and proof['codec']['rf_requests'] == 0, 'MMS codec fixtures did not pass')
    snapshot = proof['snapshot']
    roles = {row['role']: row['status'] for row in snapshot['roles']}
    operations = set(snapshot['operations'])
    require(len(snapshot['controls']) == 25, 'Incomplete typed hardware observations')
    if phase == 'baseline':
        require(roles['dialer'] != 'active' and roles['sms'] != 'active', 'An app acquired a carrier role without explicit CI selection')
        require(not ({'sendMessages', 'sendMultimediaMessages', 'activeCalls', 'callControl'} & operations), 'Protected carrier operations leaked before role consent')
    else:
        require(roles['dialer'] == 'active' and roles['sms'] == 'active', 'The emulator did not select the real qualified native roles')
        require({'readMessages', 'sendMessages', 'sendMultimediaMessages', 'activeCalls', 'callControl'} <= operations, 'Actual native permissions do not support the expected operations')
    if phase == 'sms':
        sms = proof.get('received_sms', {})
        require(sms.get('type') == 1 and sms.get('matching_rows') == 1, 'The actual incoming SMS was not persisted exactly once')
        require(sms.get('body_sha256') == hashlib.sha256(('FerroUI SMS ' + nonce).encode()).hexdigest(), 'The native SMS body differs from the modem fixture')
        messages = sms.get('bridge_messages', {})
        require(messages.get('kind') == 'messages' and messages['value']['thread_id'] == sms['thread_id'], 'Typed message query is missing or belongs to another conversation')
        require(any(row['id'] == sms['id'] and row['status'] == 'received' and not row['outgoing'] for row in messages['value']['messages']), 'The bridge did not return the real incoming SMS')
        threads = sms.get('bridge_threads', {})
        require(threads.get('kind') == 'messageThreads' and any(row['id'] == sms['thread_id'] and '5550101' in row['recipients'] for row in threads['value']), 'The native thread/canonical-address query failed')
    if phase in ('ringing', 'active'):
        require(proof['native_call_visible'] is True, 'The independent native call UI is not visible')
        require(any(call['state'] == phase and call['incoming'] for call in proof['calls']), 'Telecom did not report the real expected call state')
        presentation = proof.get('call_presentation', {})
        require(not presentation.get('error'), 'Android rejected call-notification presentation: ' + str(presentation.get('error')))
        require(presentation.get('service_type') == 4 and presentation.get('foreground_notification') is True
                and presentation.get('call_style') is True, 'The native CallStyle notification has no observed phone-call foreground owner')
    if phase == 'ended':
        require(not proof['calls'] or all(call['state'] == 'disconnected' for call in proof['calls']), 'The modem call is still live')
        presentation = proof.get('call_presentation', {})
        require(presentation.get('service_type') == 0 and presentation.get('foreground_notification') is False,
                'The final call left a foreground service or call notification behind')
    require(phase in ('baseline', 'roles', 'sms', 'ringing', 'active', 'ended'), 'Unknown phase')
    return proof['sequence']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('observation', type=Path)
    parser.add_argument('nonce')
    parser.add_argument('phase')
    parser.add_argument('after', type=int)
    args = parser.parse_args()
    try:
        print(verify(json.loads(args.observation.read_text()), args.nonce, args.phase, args.after))
    except (ValueError, KeyError, TypeError, OSError) as error:
        raise SystemExit(str(error))

if __name__ == '__main__':
    main()
