# Browser host

The browser mounts the shared `phone_runtime::PhoneRuntime` and independently
registered SDK apps into FerroUI's single-view browser lifetime. `BrowserPlatform`
implements `phone_host::platform::PlatformPlugin`. Existing HTTP, documents,
media, clipboard, camera and other browser adapters remain registered through
`ServiceRegistry`; the platform request family is owned only by the plugin.

## Application and embedded modes

The top-level page defaults to `app`. An iframe defaults to `embedded`. Choose
explicitly with `?mode=embedded` or `data-phone-mode="embedded"` on the `#out`
host element before loading `main.js`. Both modes render the same registered
applications. Embedded mode leaves its surrounding page's history alone;
application mode retains the phone's existing browser Back integration.

`launcher` and `shell` are rejected at startup. A browser window or installed PWA
does not grant an installed-application catalog, the Android HOME role, a window
manager, or a replacement desktop session. Those capabilities are absent from
the browser platform description.

After startup, the owning page receives `phonehostready` with its control object
in `event.detail`. The same object is available as `globalThis.phoneHost`:

```js
const host = globalThis.phoneHost ?? await new Promise(resolve => {
  window.addEventListener('phonehostready', event => resolve(event.detail), {once: true});
});

host.activate('ferroui-phone://app/org.ferroui.device');
host.request({kind: 'refresh'});

// Later, when the embedding owner removes this surface:
host.close();
```

`activate()` validates the `ferroui-phone` URI and queues an admitted route until
the SDK registry is mounted. The shared runtime admits only registered apps and
validated route paths. The page URL can carry the same activation in an
`activate` query parameter or a `#ferroui-phone:` fragment. `request()` submits
one validated `PlatformRequest`; it is not a promise for the operation's result.
Accepted results update the phone's platform state, application list or status
through the shared runtime. Admission errors throw synchronously. These APIs
belong to the embedding document; they do not provide a cross-origin messaging
channel or bypass iframe permissions policy.

## Browser services

| Service | Observation and execution |
| --- | --- |
| Native text sharing | Secure context, `navigator.share`, and applicable `web-share` policy. Calls the browser share UI directly; no clipboard fallback is described as a share. |
| Haptics | `navigator.vibrate` with one bounded 8–40 ms request, a visible page and the phone's vibration preference. Acceptance means the browser accepted the request, not proof that hardware vibrated. |
| Notifications | Secure context and Notification API. Permission is requested only by the explicit command. Delivery requires existing permission and the phone's notification preference; an active service worker is used when available. |
| Location | Secure context, `watchPosition`/`clearWatch`, applicable geolocation policy and the phone's location preference. Uses one validated position fix, then clears the watch. |

The JS operation starts synchronously inside the plugin's `dispatch`, before
the returned Rust future is scheduled. This preserves the initiating browser
gesture for sharing and permission APIs. The browser still applies its own
activation, document visibility, permission and device restrictions. A listed
feature describes an observed API route, not a promise that permission will be
granted or the physical device will complete the action.

There are at most eight pending operations, with non-reused task identities and
bounded request/response payloads. Location has a 15-second deadline; other
platform operations have 30 seconds. Futures dropped by runtime cancellation
remove the JS task and its timer. Location watches are also removed. A browser
permission dialog or share sheet cannot be dismissed programmatically; its
late result is discarded, and an existing dialog is not duplicated by a later
request. Cancellation before asynchronous notification registration completes
prevents that task from posting a notification.

## Lifecycle and teardown

Visibility loss suspends the shared runtime, cancels held input, camera capture,
location watches and app-owned vibration. BFCache page suspension retains the
same runtime for a later `pageshow`; focus loss alone only cancels held input,
so opening an OS prompt does not destroy the app. Permanent `pagehide` and
`phoneHost.close()` close it once.

Closing cancels platform and SDK work, releases the phone view, stops media,
camera and alarm resources, removes app-owned listeners/observers/timers and
clears the public `phoneApp`/`phoneHost` references. Already posted OS
notifications and browser permission sheets remain under browser control.
The pinned FerroUI browser backend uses one application/module per document
and has no full framework/WASM unload API. Recreate an iframe or reload the
document to create a new complete browser instance after close.

## Verification

`node --test scripts/browser-platform.test.mjs` exercises injected browser APIs:
synchronous gesture dispatch, capacity and identity checks, settings, permission
denial, cancellation, single-flight dialogs, location cleanup, deadlines and
BFCache lifecycle. Existing browser media, alarm and notification tests continue
to cover those adapters. `scripts/ui-platform-refinements.mjs` adds two checks
and a rendered Device-services capture to the normal browser UI gate. It checks
observed controls and rejected privileged requests without requesting a real
location, posting an OS notification or opening a permission/share dialog.

API behavior follows the primary specifications for
[Web Share](https://www.w3.org/TR/web-share/),
[Geolocation](https://www.w3.org/TR/geolocation/),
[Vibration](https://www.w3.org/TR/vibration/) and
[Notifications](https://notifications.spec.whatwg.org/).
