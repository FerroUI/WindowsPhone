"""Compare the deployed precache assets with a verified compiled release artifact."""
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
import argparse
import hashlib
import json
import re
import time
import urllib.parse
import urllib.request

parser = argparse.ArgumentParser()
parser.add_argument('--artifact', required=True, type=Path)
parser.add_argument('--source', required=True)
parser.add_argument('--version', required=True)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--site', default='https://ferroui.com/WindowsPhone/')
args = parser.parse_args()
root = args.artifact.resolve(strict=True)
site = args.site.rstrip('/') + '/'
if site != 'https://ferroui.com/WindowsPhone/':
    raise SystemExit('Unexpected release destination.')
if not re.fullmatch(r'[0-9a-f]{40}', args.source):
    raise SystemExit('Expected a full source commit.')
if not re.fullmatch(r'[0-9a-f]{7,40}', args.version) or not args.source.startswith(args.version):
    raise SystemExit('The build marker must abbreviate the supplied source commit.')
index = (root / 'index.html').read_text()
worker = (root / 'app-sw.js').read_text()
index_version = re.search(r'data-phone-build="([^"]+)"', index).group(1)
worker_version = re.search(r"const CACHE=CACHE_PREFIX\+'([^']+)';", worker).group(1)
if index_version != args.version or worker_version != args.version:
    raise SystemExit('Compiled artifact build markers do not identify this release.')
listed = json.loads(re.search(r'const FILES=(\[.*?\]);', worker).group(1))
paths = set()
for entry in [*listed, './index.html', './app-sw.js']:
    path = PurePosixPath(entry)
    if path.is_absolute() or '..' in path.parts:
        raise SystemExit('Unsafe compiled asset path.')
    resolved = (root / path).resolve(strict=True)
    if not resolved.is_relative_to(root) or not resolved.is_file():
        raise SystemExit('Compiled asset is outside the verified artifact.')
    paths.add(path.as_posix())
if 'phone_browser.wasm' not in paths:
    raise SystemExit('The release precache must include the application WASM.')

args.output.parent.mkdir(parents=True, exist_ok=True)
bodies = args.output.parent / (args.output.stem + '-responses')
bodies.mkdir(parents=True, exist_ok=True)
started = datetime.now(timezone.utc).isoformat()
nonce = args.version + '-' + str(time.time_ns())


def check(name):
    artifact_name = name or 'index.html'
    expected = (root / artifact_name).read_bytes()
    url = site + urllib.parse.quote(name, safe='/._-')
    request_url = url + '?release_verify=' + nonce
    row = {
        'path': name,
        'kind': 'precache_asset_or_worker' if name else 'entry_document',
        'artifact_path': artifact_name,
        'url': url,
        'request_url': request_url,
        'expected_bytes': len(expected),
        'expected_sha256': hashlib.sha256(expected).hexdigest(),
    }
    try:
        req = urllib.request.Request(request_url, headers={
            'Cache-Control': 'no-cache',
            'Accept-Encoding': 'identity',
            'User-Agent': 'WindowsPhone-release-verification',
        })
        observed = hashlib.sha256()
        size = 0
        retained = [] if name in {'', 'index.html', 'app-sw.js'} else None
        begin = time.monotonic()
        with urllib.request.urlopen(req, timeout=40) as response:
            row['status'] = response.status
            row['final_url'] = response.url
            row['headers'] = {
                key.lower(): value for key, value in response.headers.items()
                if key.lower() in {'content-type', 'cache-control', 'etag',
                                   'last-modified', 'age', 'x-github-request-id'}
            }
            for chunk in iter(lambda: response.read(1024 * 1024), b''):
                size += len(chunk)
                if size > max(len(expected) + 4096, 1024 * 1024):
                    raise ValueError('Response exceeds the expected asset size.')
                observed.update(chunk)
                if retained is not None:
                    retained.append(chunk)
        row.update(observed_bytes=size, observed_sha256=observed.hexdigest(),
                   duration_seconds=round(time.monotonic() - begin, 3))
        row['matches_compiled_artifact'] = (
            row['status'] == 200 and size == len(expected)
            and row['observed_sha256'] == row['expected_sha256'])
        required_mime_types = {
            '.html': {'text/html'},
            '.js': {'text/javascript', 'application/javascript'},
            '.mjs': {'text/javascript', 'application/javascript'},
            '.css': {'text/css'},
            '.wasm': {'application/wasm'},
        }.get(PurePosixPath(artifact_name).suffix)
        if required_mime_types is not None:
            row['required_content_types'] = sorted(required_mime_types)
            observed_mime = row['headers'].get('content-type', '').split(';')[0].strip().lower()
            row['content_type_verified'] = observed_mime in required_mime_types
        row['delivery_verified'] = (
            row['matches_compiled_artifact'] and row.get('content_type_verified', True))
        if retained is not None:
            body = b''.join(retained)
            body_name = name or 'root-document.html'
            (bodies / body_name).write_bytes(body)
            row['retained_response'] = str(bodies / body_name)
            decoded = body.decode('utf-8', errors='replace')
            pattern = (r'data-phone-build="([^"]+)"' if artifact_name == 'index.html'
                       else r"const CACHE=CACHE_PREFIX\+'([^']+)';")
            marker = re.search(pattern, decoded)
            row['build_marker'] = marker.group(1) if marker else None
        if name == 'phone_browser.wasm':
            row['wasm_mime_type_correct'] = row['content_type_verified']
    except Exception as error:
        row.update(error=str(error), matches_compiled_artifact=False, delivery_verified=False)
    return row


with ThreadPoolExecutor(max_workers=4) as pool:
    rows = list(pool.map(check, ['', *sorted(paths)]))
asset_rows = [row for row in rows if row['kind'] == 'precache_asset_or_worker']
matched = sum(row['matches_compiled_artifact'] for row in asset_rows)
wasm = next(row for row in rows if row['path'] == 'phone_browser.wasm')
passed = sum(row['delivery_verified'] for row in rows)
success = passed == len(rows)
result = {
    'schema': 1,
    'source_revision': args.source,
    'expected_build_marker': args.version,
    'site': site,
    'observed_at_utc': started,
    'asset_selection': 'Every generated service-worker precache asset plus the worker itself; '
                       'the public root document is additionally checked against index.html.',
    'asset_count': len(asset_rows),
    'matched_assets': matched,
    'http_request_count': len(rows),
    'passed_http_requests': passed,
    'success': success,
    'observations': rows,
    'qualification': 'Direct public HTTP identity checks against the verified compiled artifact. '
                     'Rendered UI behavior is supplied by the separately recorded exact-source CI run; '
                     'this check does not claim another live-site browser interaction run.',
}
args.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'record': str(args.output), 'assets': len(asset_rows), 'matched': matched,
                  'requests': len(rows), 'passed_requests': passed,
                  'success': success, 'wasm_sha256': wasm.get('observed_sha256'),
                  'failures': [{'path': row['path'] or '/',
                                'byte_identity': row['matches_compiled_artifact'],
                                'content_type_verified': row.get('content_type_verified'),
                                'error': row.get('error')}
                               for row in rows if not row['delivery_verified']]}))
raise SystemExit(0 if success else 1)
