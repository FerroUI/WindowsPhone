#!/usr/bin/env python3
"""Verify the bounded AOSP MMS dependency closure and the checked-in source provenance."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1] / 'crates/phone-android/platform/vendor/aosp-mms'
manifest = json.loads((root / 'UPSTREAM.json').read_text())
# Every vendored source must retain its upstream license and an exact checked-in hash.
rows = manifest['files']
expected = {row['path'] for row in rows}
actual = {str(path.relative_to(root / 'src')) for path in (root / 'src').rglob('*.java')}
assert expected == actual, 'The codec dependency closure differs from its provenance manifest'
for row in rows:
    data = (root / 'src' / row['path']).read_bytes()
    assert hashlib.sha256(data).hexdigest() == row['vendored_sha256'], row['path']
    assert b'Licensed under the Apache License, Version 2.0' in data, row['path']
    assert b'import com.google.android.mms' not in data, 'Hidden platform MMS classes are forbidden'
    assert b'import android.support.' not in data, 'Unpinned support-library dependency'
assert 'Apache License' in (root / 'LICENSE').read_text()
print(f'Pinned Apache-licensed AOSP MMS sources verified: {len(rows)}')
