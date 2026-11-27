//! Mounted regressions: restore real native viewers after search/detail teardown.
use super::*;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Ref, Vector, interactivity::RoutedEventArgs};
use ferroui_controls::{
    Application, Control, ScrollViewer, Window, automation::AutomationProperties,
};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use phone_domain::{AppId, DocumentAction, PhoneData};
use phone_sdk::AppRegistry;
use std::{cell::Cell, time::Duration};

fn frame() {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        FerroHeadlessPlatform::force_render_timer_tick(1);
        Dispatcher::ui_thread().run_jobs(None);
    }
}

fn fixture() -> PhoneData {
    let mut data = PhoneData::default();
    data.documents.clear();
    data.settings.lock_timeout = 0;
    for index in 0..24 {
        let mut document = OfficeDocument::blank(
            DocumentKind::Word,
            format!("Scroll document {index:02}"),
            None,
        );
        document.id = data.next_id;
        data.next_id += 1;
        document.modified = 10_000 + index;
        data.documents.push(document);
    }
    data
}

struct Mounted {
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    root: Ref<Control>,
    window: Ref<Window>,
    owner: Owner,
    saves: Rc<Cell<usize>>,
}

impl Mounted {
    fn new(motion: bool) -> Self {
        phone_controls::set_motion_enabled(motion);
        let saves = Rc::new(Cell::new(0));
        let saved = saves.clone();
        let vm = PhoneViewModel::new(fixture(), Rc::new(move |_| saved.set(saved.get() + 1)));
        let registry = Rc::new(AppRegistry::new(vec![crate::registration()]).unwrap());
        registry.bind(&vm);
        let ui = Rc::new(Session::new(vm.now(), registry));
        phone_sdk::set_current_session(&ui);
        vm.navigate(Route::App(AppId::Office));
        let owner = Owner::new_root();
        let root = owner.with(|| {
            let revision = signal(vm.revision());
            let observed = vm.clone();
            let subscription = vm
                .changed()
                .subscribe_next(move |_| revision.set(observed.revision()));
            on_cleanup(move || subscription.dispose());
            let vm = vm.clone();
            let ui = ui.clone();
            rsx! {Grid {{rerender(move || {
                revision.get();
                crate::view(vm.clone(), ui.clone())
            })}}}
            .mount()
        });
        let window = Window::new();
        window.set_width(320.0);
        window.set_height(360.0);
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        frame();
        Self {
            vm,
            ui,
            root,
            window,
            owner,
            saves,
        }
    }

    fn scroll(&self, name: &str) -> Ref<ScrollViewer> {
        self.root
            .find_descendant_of_type_where::<ScrollViewer>(false, |scroll| scroll.has_name(name))
            .unwrap_or_else(|| panic!("missing mounted {name}"))
    }

    fn editor(&self) -> Ref<TextBox> {
        self.root
            .find_descendant_of_type_where::<TextBox>(false, |editor| {
                editor.has_name("OfficeSearchInput")
            })
            .expect("standalone native search editor")
    }

    fn click(&self, label: &str) {
        self.root
            .find_descendant_of_type_where::<Button>(false, |button| {
                AutomationProperties::get_name(button).as_deref() == Some(label)
            })
            .unwrap_or_else(|| panic!("missing native command {label}"))
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        frame();
    }

    fn set_offset(&self, name: &str, value: f64) -> f64 {
        let viewer = self.scroll(name);
        viewer.set_offset(Vector::new(0.0, value));
        frame();
        viewer.offset().y
    }

    fn select(&self, index: usize) {
        self.ui.pivot.set(index);
        frame();
    }

    fn navigate(&self, route: Route) {
        let previous = self.vm.route();
        self.vm.navigate(route.clone());
        self.ui
            .registry()
            .route_changed(&self.vm, &self.ui, &previous, &route);
        frame();
    }

    fn back(&self) {
        if !self.ui.registry().back(&self.vm, &self.ui) {
            let previous = self.vm.route();
            self.vm.back();
            self.ui
                .registry()
                .route_changed(&self.vm, &self.ui, &previous, &self.vm.route());
        }
        frame();
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        self.window.close();
        self.window.set_content(None);
        self.owner.dispose();
        frame();
        phone_controls::set_motion_enabled(true);
    }
}

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0,
        "native offset {actual} versus {expected}"
    );
}

fn run(test: impl Fn() + Send + 'static) {
    // Native manipulation-property caches retain compositor resources for the
    // dispatcher thread's lifetime. Reuse the framework's assembly dispatcher
    // with its default per-test application isolation. Mounted's Drop still
    // closes every window and disposes its owner even when an assertion fails.
    static ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
        name: "phone-app-office.hub",
        test_application: None,
        test_isolation: None,
    };
    let session = HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY));
    let result = session
        .dispatch(
            move || {
                phone_controls::install_theme(&Application::current().unwrap());
                test();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30));
    result.expect("Office mounted scroll regression completes");
}

#[test]
fn search_and_document_back_restore_independent_native_hub_offsets_without_saving() {
    run(|| {
        for motion in [true, false] {
            let app = Mounted::new(motion);
            let original = serde_json::to_value(&*app.vm.data()).unwrap();
            let recent = app.set_offset("OfficeRecentScroll", 370.0);
            assert!(recent > 200.0);
            app.select(1);
            let places = app.set_offset("OfficePlacesScroll", 60.0);
            assert!(
                places > 0.0,
                "short viewport genuinely scrolls the places pane"
            );

            app.click("search");
            let editor = app.editor();
            assert!(
                editor.is_focused(),
                "search takes native editor focus on entry"
            );
            assert!(
                app.root
                    .find_descendant_of_type::<ferroui_controls::Pivot>(false)
                    .is_none(),
                "editing has no competing horizontal Pivot"
            );
            near(app.scroll("OfficeSearchScroll").offset().y, 0.0);
            app.window.key_text_input("Scroll document");
            frame();
            assert_eq!(app.ui.query.get_untracked(), "Scroll document");
            let search = app.set_offset("OfficeSearchScroll", 220.0);
            assert!(search > 100.0);
            app.window.key_text_input(" 17");
            frame();
            near(app.scroll("OfficeSearchScroll").offset().y, 0.0);
            assert!(
                editor.ptr_eq(&app.editor()) && editor.is_focused(),
                "new filter resets only its results, preserving the native editor and focus"
            );

            app.back();
            assert_eq!(app.vm.route(), Route::App(AppId::Office));
            assert_eq!(app.ui.pivot.get_untracked(), 1);
            near(app.scroll("OfficePlacesScroll").offset().y, places);
            app.select(0);
            near(app.scroll("OfficeRecentScroll").offset().y, recent);

            let id = app.vm.data().documents[12].id;
            app.navigate(Route::Document(id));
            assert!(
                app.root
                    .find_descendant_of_type_where::<ScrollViewer>(false, |viewer| viewer
                        .has_name("OfficeRecentScroll"))
                    .is_none()
            );
            app.back();
            near(app.scroll("OfficeRecentScroll").offset().y, recent);
            let viewer = app.scroll("OfficeRecentScroll");
            app.vm.set_clock(app.vm.now() + 60);
            frame();
            assert!(
                viewer.ptr_eq(&app.scroll("OfficeRecentScroll")),
                "ordinary revisions retain the native viewer"
            );
            near(viewer.offset().y, recent);
            assert_eq!(
                serde_json::to_value(&*app.vm.data()).unwrap(),
                original,
                "transient positions and navigation never enter persisted data"
            );
            assert_eq!(app.saves.get(), 0);
        }
    });
}

#[test]
fn native_restoration_clamps_changed_content_and_rejects_an_old_document_epoch() {
    run(|| {
        let app = Mounted::new(true);
        let previous = app.scroll("OfficeRecentScroll");
        let offset = app.set_offset("OfficeRecentScroll", 900.0);
        assert!(offset > 800.0);
        app.click("search");
        let removed = app
            .vm
            .data()
            .documents
            .iter()
            .skip(4)
            .map(|document| document.id)
            .collect();
        app.vm.document_action(DocumentAction::Delete(removed));
        frame();
        app.back();
        let restored = app.scroll("OfficeRecentScroll");
        let maximum = (restored.extent().height - restored.viewport().height).max(0.0);
        assert!(
            maximum < offset,
            "data deletion genuinely shrank the native extent"
        );
        near(restored.offset().y, maximum);

        // A detached old control must not overwrite the position of a new visit.
        previous.set_offset(Vector::new(0.0, 20.0));
        frame();
        app.click("search");
        app.back();
        near(app.scroll("OfficeRecentScroll").offset().y, maximum);

        let old = app.scroll("OfficeRecentScroll");
        let old_epoch = app.vm.document_epoch();
        app.vm
            .replace_data_after_save(fixture(), |_| Ok(()))
            .unwrap();
        assert_ne!(app.vm.document_epoch(), old_epoch);
        // Run native scroll work from the outgoing document before the next
        // view turn. Neither it nor any pending restoration may cross epochs.
        old.set_offset(Vector::new(0.0, 40.0));
        app.vm.navigate(Route::App(AppId::Office));
        frame();
        let current = app.scroll("OfficeRecentScroll");
        assert!(!current.ptr_eq(&old));
        near(current.offset().y, 0.0);
        app.click("search");
        app.back();
        near(app.scroll("OfficeRecentScroll").offset().y, 0.0);
    });
}

#[test]
fn changing_sort_and_normalized_filters_updates_rows_without_stale_scroll_or_focus() {
    run(|| {
        let app = Mounted::new(true);
        let offset = app.set_offset("OfficeRecentScroll", 250.0);
        let viewer = app.scroll("OfficeRecentScroll");
        let id = app.vm.data().documents[0].id;
        app.vm.document_action(DocumentAction::Rename {
            id,
            name: "Updated document".into(),
        });
        frame();
        assert!(viewer.ptr_eq(&app.scroll("OfficeRecentScroll")));
        near(viewer.offset().y, offset);
        let changed = viewer
            .find_descendant_of_type_where::<TextBlock>(false, |text| {
                text.text().as_deref() == Some("Updated document")
            })
            .expect("updated row text");
        let other = viewer
            .find_descendant_of_type_where::<TextBlock>(false, |text| {
                text.text().as_deref() == Some("Scroll document 23")
            })
            .expect("previous newest row");
        assert!(
            changed
                .translate_point(ferroui_base::Point::default(), &viewer)
                .unwrap()
                .y
                < other
                    .translate_point(ferroui_base::Point::default(), &viewer)
                    .unwrap()
                    .y,
            "modified-date sorting updates the mounted row order"
        );

        app.click("search");
        let editor = app.editor();
        editor.set_text(Some("  SCROLL document  "));
        frame();
        let filtered = app.scroll("OfficeSearchScroll");
        let offset = app.set_offset("OfficeSearchScroll", 180.0);
        editor.set_text(Some("scroll document"));
        frame();
        assert!(
            filtered.ptr_eq(&app.scroll("OfficeSearchScroll")),
            "equivalent trimmed/case-folded filters retain their native results viewer"
        );
        near(filtered.offset().y, offset);
        editor.set_text(Some("  updated DOCUMENT  "));
        frame();
        near(app.scroll("OfficeSearchScroll").offset().y, 0.0);
        assert!(
            app.root
                .find_descendant_of_type_where::<Button>(false, |button| {
                    AutomationProperties::get_name(button).as_deref()
                        == Some("Updated document.docx")
                })
                .is_some()
        );
        editor.set_text(Some("no matching documents"));
        frame();
        near(app.scroll("OfficeSearchScroll").offset().y, 0.0);
        assert!(
            app.root
                .find_descendant_of_type_where::<TextBlock>(false, |text| text.text().as_deref()
                    == Some("no documents found"))
                .is_some()
        );
        assert!(editor.ptr_eq(&app.editor()) && editor.is_focused());
        assert!(
            app.saves.get() > 0,
            "the rename follows the real document mutation path"
        );
        let saves = app.saves.get();
        app.back();
        assert_eq!(
            app.saves.get(),
            saves,
            "search dismissal does not save transient state"
        );
    });
}
