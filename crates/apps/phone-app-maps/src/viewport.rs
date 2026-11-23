//! A retained map surface. Transport is the replaceable SDK Network service;
//! the UI admits only visible tiles and never performs background prefetch.
mod gestures;
mod loader;
pub(crate) mod model;
use crate::{
    commands,
    model::{Place, Saved},
};
use ferroui_base::{
    input::Key,
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::FontWeight,
};
use ferroui_view::prelude::*;
use model::Camera;
use phone_controls::{icon, Icon};
use phone_sdk::{session::Session, AppContext};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

struct Scope {
    alive: Cell<bool>,
}
struct State {
    camera: Signal<Camera>,
    size: Signal<(f64, f64)>,
    revision: Signal<u64>,
    epoch: Cell<Option<u64>>,
    scope: RefCell<Weak<Scope>>,
    paused: Signal<bool>,
    dragging: Signal<bool>,
    deferred: Signal<bool>,
    settle_at: Cell<Option<Instant>>,
    gesture: RefCell<gestures::Gesture>,
    tiles: RefCell<loader::Tiles>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            camera: signal(Camera::default()),
            size: signal((320.0, 300.0)),
            revision: signal(0),
            epoch: Cell::new(None),
            scope: RefCell::new(Weak::new()),
            paused: signal(false),
            dragging: signal(false),
            deferred: signal(false),
            settle_at: Cell::new(None),
            gesture: RefCell::new(Default::default()),
            tiles: RefCell::new(Default::default()),
        }
    }
}
impl State {
    fn changed(&self) {
        self.revision.update(|value| *value = value.wrapping_add(1));
    }
    fn current(&self, scope: &Rc<Scope>) -> bool {
        scope.alive.get()
            && self
                .scope
                .borrow()
                .upgrade()
                .is_some_and(|active| Rc::ptr_eq(&active, scope))
    }
}
fn prepare(cx: &AppContext, saved: &Saved) -> Rc<State> {
    let state = cx.session.state::<State>();
    if state.epoch.replace(Some(cx.vm.document_epoch())) != Some(cx.vm.document_epoch()) {
        loader::cancel(&state);
        gestures::cancel(&state);
        *state.tiles.borrow_mut() = Default::default();
        let camera = saved
            .home
            .and_then(|id| saved.places.iter().find(|place| place.id == id))
            .and_then(|place| place.coordinates)
            .map(|coordinates| Camera::at(coordinates, 14.0))
            .unwrap_or_default();
        debug_assert!(camera.validate().is_ok());
        state.camera.set(camera);
        state.deferred.set(false);
        state.settle_at.set(None);
    }
    state
}
pub(crate) fn focus(cx: &AppContext, coordinates: phone_app_support::geography::Coordinates) {
    let Ok(saved) = commands::load(cx) else {
        return;
    };
    let state = prepare(cx, &saved);
    state.camera.set(Camera::at(coordinates, 14.0));
    state.deferred.set(false);
}
pub(crate) fn suspend(ui: &Session, paused: bool) {
    let state = ui.state::<State>();
    state.paused.set(paused);
    if paused {
        loader::cancel(&state);
        gestures::cancel(&state);
        state.tiles.borrow_mut().ready.clear();
        state.deferred.set(false);
        state.settle_at.set(None);
    }
}
pub(crate) fn reset(ui: &Session) {
    suspend(ui, true);
    let state = ui.state::<State>();
    state.epoch.set(None);
    *state.tiles.borrow_mut() = Default::default();
    state.camera.set(Camera::default());
    state.paused.set(false);
}
pub(crate) fn tick(cx: &AppContext) {
    let state = cx.session.state::<State>();
    if state.deferred.get_untracked()
        && state
            .settle_at
            .get()
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(180))
    {
        state.settle_at.set(None);
        state.deferred.set(false);
    }
}
fn zoom(cx: &AppContext, state: &State, delta: f64) {
    commands::cancel_location(cx);
    let size = state.size.get_untracked();
    let camera = state.camera.get_untracked();
    state.camera.set_if_changed(camera.zoom_at(
        camera.zoom.round() + delta,
        (size.0 / 2.0, size.1 / 2.0),
        size,
    ));
    state.deferred.set(false);
    state.settle_at.set(None);
}

pub(crate) fn view(
    cx: AppContext,
    saved: Saved,
    selected: Option<Place>,
    height: Option<f64>,
) -> Element {
    rerender(move || {
        let state = prepare(&cx, &saved);
        let node = use_hook(NodeRef::<Canvas>::new);
        let scope = use_hook(|| {
            Rc::new(Scope {
                alive: Cell::new(true),
            })
        });
        use_effect_with((), {
            let state = state.clone();
            let scope = scope.clone();
            move |_| {
                loader::cancel(&state);
                gestures::cancel(&state);
                *state.scope.borrow_mut() = Rc::downgrade(&scope);
                scope.alive.set(true);
                on_cleanup(move || {
                    scope.alive.set(false);
                    if state
                        .scope
                        .borrow()
                        .upgrade()
                        .is_some_and(|active| Rc::ptr_eq(&active, &scope))
                    {
                        loader::cancel(&state);
                        gestures::cancel(&state);
                        state.tiles.borrow_mut().ready.clear();
                        *state.scope.borrow_mut() = Weak::new();
                    }
                });
            }
        });
        state.revision.get();
        let camera = state.camera.get();
        let size = state.size.get();
        let loading_allowed = !state.paused.get() && !state.dragging.get() && !state.deferred.get();
        use_effect_with((camera, size, loading_allowed, cx.vm.document_epoch()), {
            let state = state.clone();
            let scope = scope.clone();
            let cx = cx.clone();
            move |_| {
                if state.current(&scope) {
                    loader::synchronize(&state, &cx, &scope);
                }
            }
        });
        let tiles = camera.visible(size);
        let mut surface=canvas().name("MapCanvas").key("map-canvas").node_ref(node).background("#E7E6DF").clip_to_bounds(true).focusable(true)
            .automation_properties_name("Interactive map").automation_properties_help_text("Drag to pan. Pinch or use plus and minus to zoom. Arrow keys pan. Tap the map to choose a place.");
        for (index, tile) in tiles.into_iter().enumerate() {
            let picture=state.tiles.borrow().ready.get(&tile.key).map(|image|image.fit(format!("MapTileImage-{}-{}-{}-{index}",tile.key.zoom,tile.key.x,tile.key.y))).unwrap_or_else(||rsx!{Border{background:"#E7E6DF",border_brush:"#D9D8D1",border_thickness:0.5}});
            surface = surface.child(
                border()
                    .name(format!(
                        "MapTile-{}-{}-{}-{index}",
                        tile.key.zoom, tile.key.x, tile.key.y
                    ))
                    .key(format!(
                        "map-tile-{}-{}-{}-{index}",
                        tile.key.zoom, tile.key.x, tile.key.y
                    ))
                    .canvas_left(tile.x)
                    .canvas_top(tile.y)
                    .width(tile.size + 0.5)
                    .height(tile.size + 0.5)
                    .is_hit_test_visible(false)
                    .child(picture),
            );
        }
        let size_state = state.clone();
        let press = state.clone();
        let press_cx = cx.clone();
        let movement = state.clone();
        let release = state.clone();
        let release_cx = cx.clone();
        let lost = state.clone();
        let wheel = state.clone();
        let wheel_cx = cx.clone();
        let keys = state.clone();
        let key_cx = cx.clone();
        surface = surface
            .on_size_changed(move |event| {
                let size = event.new_size();
                if size.width > 0.0 && size.height > 0.0 {
                    size_state
                        .size
                        .set_if_changed(model::dimensions((size.width, size.height)));
                }
            })
            .on_pointer_pressed(move |event| {
                commands::cancel_location(&press_cx);
                gestures::pressed(&press, node, event);
            })
            .on_pointer_moved(move |event| gestures::moved(&movement, node, event))
            .on_pointer_released(move |event| {
                gestures::released(&release, node, &release_cx, event)
            })
            .on_pointer_capture_lost(move |event| gestures::lost(&lost, event.pointer().id()))
            .on_pointer_wheel_changed(move |event| {
                let Some(canvas) = node.get_untracked() else {
                    return;
                };
                let point = event.get_position(Some(&canvas));
                let delta = event.delta().y;
                if delta == 0.0 {
                    return;
                }
                commands::cancel_location(&wheel_cx);
                wheel.deferred.set(true);
                wheel.settle_at.set(Some(Instant::now()));
                let camera = wheel.camera.get_untracked();
                wheel.camera.set_if_changed(camera.zoom_at(
                    camera.zoom + delta.signum(),
                    (point.x, point.y),
                    wheel.size.get_untracked(),
                ));
                event.set_handled(true);
            })
            .on_key_down(move |event| {
                let size = keys.size.get_untracked();
                let camera = keys.camera.get_untracked();
                let next = match event.key {
                    Key::Left => Some(camera.pan(80.0, 0.0, size)),
                    Key::Right => Some(camera.pan(-80.0, 0.0, size)),
                    Key::Up => Some(camera.pan(0.0, 80.0, size)),
                    Key::Down => Some(camera.pan(0.0, -80.0, size)),
                    Key::Add | Key::OemPlus => {
                        zoom(&key_cx, &keys, 1.0);
                        None
                    }
                    Key::Subtract | Key::OemMinus => {
                        zoom(&key_cx, &keys, -1.0);
                        None
                    }
                    _ => return,
                };
                if let Some(next) = next {
                    commands::cancel_location(&key_cx);
                    keys.camera.set_if_changed(next);
                }
                event.set_handled(true);
            });
        let mut markers = canvas().clip_to_bounds(true);
        for place in saved
            .places
            .iter()
            .chain(selected.iter())
            .filter(|place| place.coordinates.is_some())
        {
            if selected
                .as_ref()
                .is_some_and(|selected| selected.id != 0 && selected.id == place.id)
                && saved.places.iter().any(|saved| std::ptr::eq(saved, place))
            {
                continue;
            }
            let Some((x, y)) = camera.position(place.coordinates.unwrap(), size) else {
                continue;
            };
            if !(-18.0..=size.0 + 18.0).contains(&x) || !(-40.0..=size.1 + 40.0).contains(&y) {
                continue;
            }
            let open = cx.clone();
            let place = place.clone();
            let label = place.name.clone();
            let id = place.id;
            markers=markers.child(rsx!{Button{name:format!("MapPlace-{id}"),key:format!("map-place-{id}"),class:"phone-flat",Canvas.left:x-18.0,Canvas.top:y-38.0,width:36.0,height:40.0,min_height:0.0,padding:0.0,automation_properties_name:format!("Map pin {label}"),tool_tip_tip:label,
                on_click:move|event|{commands::select(&open,place.clone());event.set_handled(true);},Border{background:phone_sdk::common::accent(&cx.vm).as_str(),corner_radius:18.0,border_brush:"White",border_thickness:2.0,{icon(Icon::Location,22.0,"White")}}
            }});
        }
        let plus = cx.clone();
        let plus_state = state.clone();
        let minus = cx.clone();
        let minus_state = state.clone();
        let retry = cx.clone();
        let retry_state = state.clone();
        let attribution = cx.clone();
        let (loading, failed) = {
            let tiles = state.tiles.borrow();
            (tiles.loading(), !tiles.failed.is_empty())
        };
        let (scale, label) = camera.scale();
        let content = rsx! {Grid{name:"MapViewport",class:"phone-native-input",clip_to_bounds:true,min_height:180.0,max_width:model::MAX_DIMENSION,max_height:model::MAX_DIMENSION,horizontal_alignment:H::Stretch,
            {surface}{markers}
            Border{horizontal_alignment:H::Left,vertical_alignment:V::Top,margin:6.0,padding:(5.0,3.0),background:"#EEFFFFFF",max_width:(size.0-68.0).max(80.0),is_hit_test_visible:false,
                TextBlock{name:"MapCoordinates",text:format!("{} · zoom {:.1}",camera.center.label(),camera.zoom),font_size:11.0,foreground:"#151515"}}
            StackPanel{horizontal_alignment:H::Right,vertical_alignment:V::Top,margin:(0.0,7.0,7.0,0.0),spacing:4.0,
                Button{name:"MapZoomIn",class:"phone-flat",width:42.0,height:42.0,min_height:0.0,padding:0.0,background:"#F8FFFFFF",is_enabled:camera.zoom<model::MAX_ZOOM,automation_properties_name:"Zoom map in",on_click:move|_|zoom(&plus,&plus_state,1.0),TextBlock{text:"+",font_size:29.0,foreground:"Black",horizontal_alignment:H::Center}},
                Button{name:"MapZoomOut",class:"phone-flat",width:42.0,height:42.0,min_height:0.0,padding:0.0,background:"#F8FFFFFF",is_enabled:camera.zoom>0.0,automation_properties_name:"Zoom map out",on_click:move|_|zoom(&minus,&minus_state,-1.0),TextBlock{text:"−",font_size:29.0,foreground:"Black",horizontal_alignment:H::Center}}
            }
            if failed{Button{name:"MapRetry",class:"phone-flat",background:"#F5FFFFFF",horizontal_alignment:H::Center,vertical_alignment:V::Center,automation_properties_name:"Retry unavailable map tiles",on_click:move|_|loader::retry(&retry_state,&retry),TextBlock{text:"Some map tiles are unavailable.\nTap to retry.",font_size:17.0,foreground:"#151515",text_wrapping:ferroui_base::media::TextWrapping::Wrap}}}
            if loading&&!failed{Border{horizontal_alignment:H::Left,vertical_alignment:V::Bottom,margin:(6.0,0.0,0.0,35.0),background:"#EEFFFFFF",padding:4.0,is_hit_test_visible:false,TextBlock{name:"MapLoading",text:"loading map…",font_size:12.0,foreground:"#151515"}}}
            Border{horizontal_alignment:H::Left,vertical_alignment:V::Bottom,margin:(7.0,0.0,0.0,7.0),padding:(5.0,3.0),background:"#EEFFFFFF",is_hit_test_visible:false,
                StackPanel{spacing:3.0,TextBlock{text:label,font_size:10.0,foreground:"Black",font_weight:FontWeight::SemiBold},Border{width:scale,height:4.0,border_thickness:(1.0,0.0,1.0,1.0),border_brush:"Black",horizontal_alignment:H::Left}}}
            Button{name:"MapAttribution",class:"phone-flat",horizontal_alignment:H::Right,vertical_alignment:V::Bottom,background:"#F8FFFFFF",min_height:28.0,padding:(5.0,3.0),automation_properties_name:"OpenStreetMap attribution",on_click:move|_|attribution.vm.request(phone_state::HostRequest::OpenUrl{url:phone_sdk::host::maps::ATTRIBUTION_URL.into()}),TextBlock{text:phone_sdk::host::maps::ATTRIBUTION,font_size:11.0,foreground:"#151515"}}
        }};
        if let Some(height) = height {
            border().height(height).child(content).into()
        } else {
            content
        }
    })
}
