#!/usr/bin/env python3
"""Generate a declarative FerroUI phone app without changing composition files."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys


CARGO_NAME = re.compile(r"phone-app-[a-z][a-z0-9]*(?:-[a-z0-9]+)*\Z")
APP_ID = re.compile(r"[a-z][a-z0-9]*(?:[.-][a-z0-9]+)+\Z")


def rust_string(value: str) -> str:
    # JSON string quoting is valid Rust for the admitted printable strings.
    return json.dumps(value, ensure_ascii=False)


def files(name: str, title: str, app_id: str) -> dict[str, str]:
    manifest = f'''[package]
name = "{name}"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false
build = "build.rs"

[dependencies]
phone-sdk.workspace = true
ferroui-base.workspace = true
ferroui-controls.workspace = true
ferroui-view.workspace = true
serde = {{ workspace = true, features = ["derive"] }}

[build-dependencies]
ferroui-build.workspace = true
'''
    source = '''//! Independently registered FerroUI phone application.
use phone_sdk::prelude::*;
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = @ID@;
const TITLE: &str = @TITLE@;

#[derive(Serialize, Deserialize)]
struct SavedState {
    schema: u32,
    count: u64,
}
impl Default for SavedState {
    fn default() -> Self { Self { schema: 1, count: 0 } }
}

pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, TITLE, Icon::Note)
            .category(AppCategory::Utility),
        view,
    )
}

fn view(cx: AppContext) -> Element {
    let storage = cx.storage();
    let saved = match storage.read::<SavedState>() {
        Ok(saved) if saved.schema == 1 => saved,
        Ok(_) => return empty(TITLE, "This saved data version is not supported."),
        Err(error) => return empty(TITLE, &error),
    };
    let model = cx.vm.clone();
    let body = rsx! { StackPanel { margin:20.0, spacing:18.0,
        {title(TITLE)}
        {text(&saved.count.to_string(),72.0)}
        {muted("This value belongs to this application's persistent namespace.",18.0)}
        {flat("increment",move || {
            let result = (|| {
                let mut saved = storage.read::<SavedState>()?;
                if saved.schema != 1 { return Err("Unsupported saved data version.".into()); }
                saved.count = saved.count.checked_add(1).ok_or("The counter is at its maximum.")?;
                storage.write(&saved)
            })();
            if let Err(error) = result { model.set_status(error); }
        })}
    }};
    page(TITLE, scroll(body), Element::Empty)
}
'''.replace("@ID@", rust_string(app_id)).replace("@TITLE@", rust_string(title))
    build = '''fn main() {
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .run();
}
'''
    readme = f'''# {title}

A statically linked phone app built with FerroUI and `phone-sdk`.

- App identity: `{app_id}`.
- Entry point: `registration()`.
- Data: a versioned counter in its own `AppStorage` namespace.
- Host capabilities: none requested.

Add this crate to the composition dependencies, then pass
`{name.replace('-', '_')}::registration()` to
`phone_ui::create_view_with_apps`. See the workspace's `docs/sdk.md`.
'''
    return {"Cargo.toml": manifest, "build.rs": build, "src/lib.rs": source, "README.md": readme}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("name", help="Cargo name, e.g. phone-app-countdown")
    parser.add_argument("--title", required=True, help="Title displayed in the app list")
    parser.add_argument("--id", required=True, dest="app_id", help="Stable identity, e.g. example.countdown")
    parser.add_argument("--output", type=Path, help="New crate directory; defaults to crates/apps/NAME")
    parser.add_argument("--dry-run", action="store_true", help="Print the file plan without writing")
    args = parser.parse_args()
    if not CARGO_NAME.fullmatch(args.name) or len(args.name) > 64:
        parser.error("name must be a lowercase phone-app-* Cargo identifier of at most 64 characters")
    if not APP_ID.fullmatch(args.app_id) or len(args.app_id) > 80:
        parser.error("id must be a lowercase namespaced identifier of at most 80 characters")
    if args.app_id.startswith("phone."):
        parser.error("the phone.* namespace is reserved for built-in applications")
    if not 1 <= len(args.title.strip()) <= 80 or not args.title.isprintable():
        parser.error("title must contain 1–80 printable characters")
    workspace = Path(__file__).resolve().parent.parent
    output = args.output or workspace / "crates" / "apps" / args.name
    output = output.absolute()
    if output.exists() or output.is_symlink():
        parser.error(f"refusing to overwrite existing path: {output}")
    generated = files(args.name, args.title.strip(), args.app_id)
    plan = {"crate": args.name, "id": args.app_id, "directory": str(output), "files": list(generated)}
    print(json.dumps(plan, indent=2, ensure_ascii=False))
    if not args.dry_run:
        output.mkdir(parents=True, exist_ok=False)
        try:
            for relative, content in generated.items():
                target = output / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open("x", encoding="utf-8", newline="\n") as file:
                    file.write(content)
        except Exception:
            print(f"Creation stopped; inspect partial output at {output}.", file=sys.stderr)
            raise
    print("\nComposition dependency (adjust the path for the consuming crate):")
    print(f'{args.name} = {{ path = "../apps/{args.name}" }}')
    print("\nAdd to the registry passed to create_view_with_apps:")
    print(f'{args.name.replace("-", "_")}::registration(),')
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
