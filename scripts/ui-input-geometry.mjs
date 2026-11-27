import assert from "node:assert/strict";

// Native PivotItem owns the accessible TabItem name. Its separate rendered
// PivotHeaderItem has content text and a non-semantic peer; pointer tests must
// target that real header without inventing an automation name for it.
export function buttonLabelMatches(element, label) {
  return /(?:Button|ToggleSwitch|CheckBox|PivotHeaderItem)$/.test(element.type) &&
    (element.automationName === label ||
      (/PivotHeaderItem$/.test(element.type) && element.text === label));
}

// inspectView.hit uses the transformed local centre, while its x/y and
// width/height expose transformed origin and logical size. Their arithmetic
// centre is a pointer target only after native page projections detach.
export function inputGeometryReady(view) {
  if (!Array.isArray(view?.elements)) return false;
  const named = name => view.elements.find(element => element.name === name);
  const page = named("PhonePage"), outgoing = named("PhoneOutgoingPage");
  const image = named("PhoneOutgoingImage"), picker = named("PhonePickerSurface");
  return (!page || (page.projection === null && page.opacity === 1)) &&
    (!outgoing || (outgoing.projection === null && outgoing.opacity === 0)) &&
    !image?.imageSource &&
    (!picker || (picker.projection === null && picker.opacity === 1 && picker.hitTestVisible));
}

export function inputTarget(view, match, label) {
  const found = view.elements.filter(element => element.hit && element.enabled !== false && match(element));
  assert(found.length <= 1, `Ambiguous control: ${label} (${found.length})`);
  return found[0];
}

function sameGeometry(before, after) {
  return after && ["type", "name", "automationName"].every(key => before[key] === after[key]) &&
    ["x", "y", "width", "height"].every(key => Number.isFinite(after[key]) &&
      Math.abs(before[key] - after[key]) < 0.1) &&
    ["x", "y", "width", "height"].every(key =>
      Number.isFinite(after.renderedBounds?.[key]) &&
      Math.abs(before.renderedBounds?.[key] - after.renderedBounds[key]) < 0.1);
}

// Readiness and target lookup share each snapshot. Reacquisition after actual
// rendered frames also rejects a pending entrance or moving Pivot/layout target.
export async function stableInputTarget({view, snapshot, frame, match, label}) {
  const before = view ?? await snapshot();
  if (!inputGeometryReady(before)) return;
  const candidate = inputTarget(before, match, label);
  if (!candidate) return;
  await frame();
  const after = await snapshot();
  if (!inputGeometryReady(after)) return;
  const target = inputTarget(after, match, label);
  return sameGeometry(candidate, target) ? target : undefined;
}
