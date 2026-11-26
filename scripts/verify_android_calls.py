#!/usr/bin/env python3
"""Verify the guarded real CallLog fixture; this is provider evidence, not UI input."""
import argparse
import json
import re
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def missed(proof, name, count, acknowledgement=None):
    receipt = proof[name]
    require(receipt.get('kind') == 'device' and receipt['response']['kind'] == 'missedCalls',
            f'{name} is not a real typed missed-call response')
    value = receipt['response']['value']
    require(type(value['revision']) is int and value['revision'] > 0 and type(value['count']) is int
            and value['count'] == count, f'{name} has an incorrect independent missed-call count')
    if acknowledgement is not None:
        token = value.get('acknowledgement', {})
        require(token.get('count') == acknowledgement and isinstance(token.get('token'), str)
                and re.fullmatch(r'[0-9a-fA-F-]{36}', token['token']), f'{name} has no bounded provider receipt')
    return value


def row_map(rows):
    require(isinstance(rows, list) and len(rows) <= 512, 'Unbounded CallLog row evidence')
    result = {}
    for row in rows:
        require(all(type(row.get(key)) is int for key in ('id', 'timestamp', 'type'))
                and row['id'] > 0 and row['timestamp'] > 0
                and all(row.get(key) is None or (type(row[key]) is int and row[key] in (0, 1)) for key in ('new', 'is_read')),
                'Invalid actual CallLog flags')
        require(row['id'] not in result, 'Duplicate CallLog row identity')
        result[row['id']] = row
    return result


def verify(proof, nonce):
    require(proof.get('schema') == 1 and re.fullmatch(r'[0-9a-f]{32}', nonce) and proof.get('nonce') == nonce,
            'Foreign CallLog fixture observation')
    require(proof.get('kind') == 'real_calllog_provider_fixture' and proof.get('result') == 'passed' and proof.get('phase') == 'complete'
            and not proof.get('error'), proof.get('error') or 'The native CallLog fixture did not pass')
    require(proof.get('rf_requests') == 0, 'The fixture must not initiate calls or RF traffic')
    guards = proof['guards']
    require(all(guards.get(key) is True for key in ('debug', 'emulator', 'foreground', 'dialer', 'writable', 'nonce_marker')),
            'The fixture lacks its native debug/emulator/consent/foreground guards')
    require(guards.get('hardware') in ('ranchu', 'goldfish') and
            guards.get('product', '').startswith(('sdk_', 'google_sdk')) and 'x86_64' in guards.get('abis', []),
            'Unexpected native emulator identity')
    require(type(proof['started_at']) is int and type(proof['completed_at']) is int
            and 0 < proof['started_at'] <= proof['completed_at'], 'Invalid fixture observation lifetime')
    values = [missed(proof, name, count, acknowledgement) for name, count, acknowledgement in (
        ('initial_receipt', 0, None), ('captured_receipt', 2, 2), ('before_ack_receipt', 3, 3),
        ('after_ack_receipt', 1, 1), ('after_repeat_receipt', 1, 1), ('cleanup_receipt', 0, None))]
    require(all(a['revision'] < b['revision'] for a, b in zip(values, values[1:])), 'Stale provider revisions')
    inserted = proof['inserted_rows']
    require(len(inserted) == 3 and len({row['id'] for row in inserted}) == 3, 'Exactly three identified fixture rows are required')
    first, second, later = [row['id'] for row in inserted]
    require(all(type(value) is int and value > 0 for value in (first, second, later)), 'Invalid fixture IDs')
    require([row['number'] for row in inserted] == ['5550141', '5550142', '5550143'], 'Unexpected fixture phone numbers')
    require(inserted[0]['timestamp'] < inserted[1]['timestamp'] < inserted[2]['timestamp'], 'The later fixture call must have its own later timestamp')
    history = proof['history_receipt']
    require(proof['history_limit'] == 1 and history.get('kind') == 'device'
            and history['response']['kind'] == 'callHistory', 'Missing actual one-row call-history query')
    page = history['response']['value']
    require(len(page) == 1 and page[0]['id'] == f'call-log:{second}', 'The one-row history page must remain independent of missed count 2')
    initial, before, after, repeated, final = [row_map(proof[name]) for name in (
        'rows_initial', 'rows_before_ack', 'rows_after_ack', 'rows_after_repeat', 'rows_after_cleanup')]
    require(not any(row['type'] == 3 and row['new'] == 1 for row in initial.values()),
            'The fixture must refuse preexisting unacknowledged missed calls')
    require(set(before) == set(initial) | {first, second, later} and not set(initial) & {first, second, later},
            'Unrelated or unidentified rows entered the fixture')
    require(set(after) == set(before) and repeated == after, 'Acknowledgment deleted rows or repeated-token reuse changed data')
    for index, entry in enumerate(inserted):
        row = before[entry['id']]
        require(row['type'] == 3 and row['new'] == 1 and row['timestamp'] == entry['timestamp']
                and row['is_read'] == (1 if index == 1 else 0), 'Fixture NEW/IS_READ setup was not observed')
    for identifier, old in before.items():
        expected = dict(old)
        if identifier in (first, second):
            expected['new'] = 0
        require(after[identifier] == expected, 'Only the captured NEW flags may change; IS_READ and other rows must remain intact')
    require(after[later]['new'] == 1, 'The later missed call was acknowledged by an earlier receipt')
    repeat = proof['repeat_receipt']
    require(repeat.get('kind') == 'error' and bool(repeat.get('message')), 'The consumed receipt must be rejected on repeat')
    cleanup = proof['cleanup']
    require(cleanup.get('errors') == [] and cleanup.get('remaining_ids') == []
            and sorted(cleanup['deleted_ids']) == sorted((first, second, later)) and final == initial,
            'Cleanup did not remove exactly the identified fixture rows and restore the initial flags')
    return 'Real CallLog fixture passed: count 3→1, two captured NEW flags only, IS_READ unchanged, stale token rejected, three owned rows removed.'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('observation', type=Path)
    parser.add_argument('nonce')
    args = parser.parse_args()
    try:
        print(verify(json.loads(args.observation.read_text()), args.nonce))
    except (ValueError, KeyError, TypeError, OSError) as error:
        raise SystemExit(str(error))
