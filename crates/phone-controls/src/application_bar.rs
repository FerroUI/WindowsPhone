//! Phone application commands: four primary circles and a separate ellipsis.
//! The SDK supplies expansion state, so Back, navigation and app-bar input share
//! one owner. Standalone bars retain their own expansion state when no host is
//! connected, so overflow commands remain reachable in a control gallery too.
use crate::{Icon, palette};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::media::{TextTrimming, TextWrapping};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::element::{Handler, PropEntry, PropValue};
use ferroui_view::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
#[path = "application_bar/layout_tests.rs"]
mod layout_tests;

/// Original ApplicationBar.MiniSize is 30 at the 480-pixel phone reference.
/// A collapsed Panorama bar reserves this extent; expansion overlays content.
pub const APP_BAR_MINIMIZED_HEIGHT: f64 = crate::reference_pixels(30.0);

#[derive(Clone)]
struct ExpansionController {
    read: Rc<dyn Fn() -> bool>,
    write: Rc<dyn Fn(bool)>,
}

ferroui_base::ferro_thread_local! {
    static EXPANSION: RefCell<Option<ExpansionController>> = const { RefCell::new(None) };
}

/// Connect bar expansion to the host's transient overlay state. Callbacks must
/// use weak ownership: a control theme must never keep a disposed app alive.
pub fn set_application_bar_controller(read: Rc<dyn Fn() -> bool>, write: Rc<dyn Fn(bool)>) {
    EXPANSION.with(|slot| *slot.borrow_mut() = Some(ExpansionController { read, write }));
}

fn controller() -> Option<ExpansionController> {
    EXPANSION.with(|slot| slot.borrow().clone())
}

/// Circular icon with an expanded-only label. The ellipsis is deliberately a
/// separate, uncircled affordance and does not consume one of the four slots.
pub fn action(kind: Icon, label: &str, on_click: impl Fn() + 'static) -> Element {
    let more = matches!(kind, Icon::More);
    if more {
        return rsx! {Button {
            name:"AppBarMore",class:"phone-more", min_width:44.0, min_height:60.0,
            horizontal_alignment:H::Stretch, vertical_alignment:V::Top,
            automation_properties_name:label, tool_tip_tip:label,
            on_click:move |event| {event.set_handled(true);on_click();},
            {crate::icons::action_glyph(Icon::More,24.0,"more")}
        }};
    }
    let host = controller();
    rsx! {Button {
        name:format!("AppBarAction-{label}"),class:"phone-action", min_width:44.0, min_height:60.0, padding:(2.0,10.0),
        horizontal_alignment:H::Stretch, vertical_alignment:V::Top,
        automation_properties_name:label, tool_tip_tip:label,
        on_click:move |event| {
            event.set_handled(true);
            if let Some(host)=&host {if (host.read)() {(host.write)(false);}}
            on_click();
        },
        StackPanel {spacing:5.0, horizontal_alignment:H::Center,
            Border {name:format!("AppBarCircle-{label}"),class:"phone-action-circle",width:40.0,height:40.0,corner_radius:40.0,
                border_thickness:2.5,
                {crate::icons::action_glyph(kind,26.0*5.0/6.0,label)}
            }
            TextBlock {name:format!("AppBarLabel-{label}"),class:"phone-action-label",text:label,font_size:11.0,
                horizontal_alignment:H::Center,
                text_wrapping:TextWrapping::NoWrap,max_lines:1,
                text_trimming:<dyn TextTrimming>::character_ellipsis(),max_width:76.0}
        }
    }}
}

/// Close before a command's own callback: a command may open another surface or
/// navigate synchronously. Attaching first also works with the handled click
/// events produced by `flat`. The weak control is only used for cleanup.
fn close_on_command(control: &ferroui_base::Ref<Control>, host: ExpansionController) {
    let direct_command = control.cast::<Button>().is_some();
    let token = control.add_handler_with(
        Button::click_event(),
        move |_, event| {
            let Some(button) = event.source().and_then(|source| source.cast::<Button>()) else {
                return;
            };
            // Declarative commands already dismissed at the button before
            // invoking their callback. The bar's fallback must not close a
            // replacement menu that the callback deliberately opened.
            if !direct_command && button.classes().contains("phone-app-bar-command") {
                return;
            }
            if !button.classes().contains("phone-more") && (host.read)() {
                (host.write)(false);
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let weak = control.downgrade();
    on_cleanup(move || {
        if let Some(control) = weak.upgrade() {
            control.remove_handler(Button::click_event(), token);
        }
    });
}

fn dismiss_before_invoke(mut element: Element, host: &ExpansionController) -> Element {
    match &mut element {
        Element::Control(control) => {
            if (control.class.type_id)() == std::any::TypeId::of::<Button>() {
                control.classes.push("phone-app-bar-command".into());
                let host = host.clone();
                control.handlers.insert(
                    0,
                    Handler::attached_again(Rc::new(move |control| {
                        close_on_command(control, host.clone());
                    })),
                );
            } else {
                control.children = std::mem::take(&mut control.children)
                    .into_iter()
                    .map(|child| dismiss_before_invoke(child, host))
                    .collect();
            }
        }
        Element::Fragment(children) | Element::List(children) => {
            *children = std::mem::take(children)
                .into_iter()
                .map(|child| dismiss_before_invoke(child, host))
                .collect();
        }
        _ => {}
    }
    element
}

fn is_more(element: &Element) -> bool {
    matches!(element, Element::Control(control) if control.classes.iter().any(|class| class == "phone-more"))
}

/// Preserve the real command and its callback when an app supplies more than
/// four primary actions; overflow becomes a text command, never a hidden action.
fn overflow_command(mut element: Element) -> Element {
    if let Element::Control(control) = &mut element {
        let label = control.props.iter().find_map(|entry| {
            if entry.property != AutomationProperties::name_property().as_property() {
                return None;
            }
            match &entry.value {
                PropValue::Value(value) => value
                    .as_ref()
                    .downcast_ref::<Option<String>>()
                    .cloned()
                    .flatten(),
                _ => None,
            }
        });
        if let Some(label) = label {
            control.classes = vec!["phone-flat".into()];
            control.children = vec![rsx! {TextBlock {text:label,font_size:22.0,
            text_wrapping:TextWrapping::Wrap}}];
        }
    }
    element
}

/// Display up to four primary actions plus the edge ellipsis. Existing callers
/// may include `action(Icon::More, ...)`; otherwise one is supplied, including
/// bars that have labels but no secondary commands. Without a host controller,
/// expansion is retained locally; changing `expanded` synchronizes that state.
pub fn app_bar(actions: Vec<Element>, expanded: bool, secondary: Vec<Element>) -> Element {
    app_bar_mode(actions, expanded, secondary, false)
}

/// Panorama command surface: only the ellipsis is visible while minimized.
/// Expanded commands keep the same native controls, target widths, labels, and
/// dismissal ownership as a regular application bar, then return to mini size.
pub fn minimized_app_bar(
    actions: Vec<Element>,
    expanded: bool,
    secondary: Vec<Element>,
) -> Element {
    app_bar_mode(actions, expanded, secondary, true)
}

fn app_bar_mode(
    actions: Vec<Element>,
    expanded: bool,
    secondary: Vec<Element>,
    minimized: bool,
) -> Element {
    if let Some(host) = controller() {
        return render_app_bar(
            actions,
            expanded || (host.read)(),
            secondary,
            host,
            minimized,
        );
    }
    rerender(move || {
        let open = use_signal(|| expanded);
        use_effect_with(expanded, move |next| {
            if open.get_untracked() != *next {
                open.set(*next);
            }
        });
        let expanded = open.get();
        let host = ExpansionController {
            read: Rc::new(move || open.is_alive() && open.get_untracked()),
            write: Rc::new(move |value| {
                if open.is_alive() && open.get_untracked() != value {
                    open.set(value);
                }
            }),
        };
        render_app_bar(
            actions.clone(),
            expanded,
            secondary.clone(),
            host,
            minimized,
        )
    })
}

fn render_app_bar(
    actions: Vec<Element>,
    expanded: bool,
    secondary: Vec<Element>,
    host: ExpansionController,
    minimized: bool,
) -> Element {
    let p = palette();
    let mut primary = Vec::new();
    let mut more = None;
    let mut overflow = Vec::new();
    for action in actions {
        if is_more(&action) && more.is_none() {
            more = Some(action);
        } else if primary.len() < 4 {
            primary.push(dismiss_before_invoke(action, &host));
        } else {
            overflow.push(dismiss_before_invoke(overflow_command(action), &host));
        }
    }
    if more.is_none() {
        let host = host.clone();
        more = Some(action(Icon::More, "more", move || {
            (host.write)(!(host.read)())
        }));
    }
    let collapsed_mini = minimized && !expanded;
    let height = if collapsed_mini {
        APP_BAR_MINIMIZED_HEIGHT
    } else {
        crate::APP_BAR_HEIGHT
    };
    // Keep the supplied ellipsis callback and native Button identity, changing
    // only its row extent. Primary actions retain their ordinary touch targets.
    if let Some(Element::Control(control)) = more.as_mut() {
        let property = Layoutable::min_height_property().as_property();
        control.props.retain(|entry| entry.property != property);
        control.props.push(PropEntry {
            property,
            value: PropValue::Value(Rc::new(height)),
        });
    }
    overflow.extend(
        secondary
            .into_iter()
            .map(|command| dismiss_before_invoke(command, &host)),
    );
    let count = primary.len();
    let mut icons = uniform_grid()
        .name("AppBarPrimaryCommands")
        .columns(count.max(1) as i32)
        // Center alignment measures the circle's 44-unit minimum, then jumps
        // to the caption's desired width when labels appear. Stretch keeps
        // the same equal command slots in both states; MaxWidth centers the
        // bounded group while allowing four slots to fit a narrow viewport.
        .horizontal_alignment(H::Stretch)
        .max_width(count as f64 * 76.0);
    for action in primary {
        icons = icons.child(action);
    }
    let mut body = stack_panel().child(rsx! {Grid {
        name:"AppBarCommands",column_definitions:"44,*,44",min_height:height,
        Border {Grid.column:1,is_visible:!collapsed_mini,{icons}}
        Border {Grid.column:2,vertical_alignment:V::Top,{more.unwrap_or(Element::Empty)}}
    }});
    if expanded && !overflow.is_empty() {
        let mut menu = stack_panel()
            .key("app-bar-menu")
            .spacing(3.0)
            .margin((20.0, 8.0, 20.0, 18.0))
            .on_loaded(|event| {
                if let Some(control) = event.source().and_then(|source| source.cast::<Control>()) {
                    crate::reveal(&control);
                }
            });
        for action in overflow {
            menu = menu.child(action);
        }
        body = body.child(rsx! {ScrollViewer {name:"AppBarMenuScroll",
        max_height:crate::tokens::app_menu_height(),
        horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,
        vertical_scroll_bar_visibility:ScrollBarVisibility::Auto,{menu}}});
    }
    border()
        .name("PhoneApplicationBar")
        .class("phone-app-bar")
        .class(if minimized { "minimized" } else { "normal" })
        .class(if expanded { "expanded" } else { "collapsed" })
        .background(p.surface)
        .min_height(height)
        // Commands supplied as dynamic components cannot be inspected above;
        // the bar observes their handled, bubbling click as a fallback.
        .with(move |control| close_on_command(control.upcast_ref(), host.clone()))
        .child(body)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::FerroLocator;
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_base::platform::IPlatformRenderInterface;
    use ferroui_base::reactive::IDisposable;
    use std::cell::Cell;

    fn render_scope() -> Rc<dyn IDisposable> {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(ferroui_headless::testing::headless_platform_render_interface());
        scope
    }

    #[test]
    fn overflow_retains_the_native_command_and_accessible_label() {
        let _render = render_scope();
        EXPANSION.with(|slot| slot.borrow_mut().take());
        let calls = Rc::new(Cell::new(0));
        let invoked = calls.clone();
        let (control, owner) = overflow_command(action(Icon::Save, "save a copy", move || {
            invoked.set(invoked.get() + 1);
        }))
        .mount_root();
        let button = control.cast::<Button>().unwrap();
        assert!(button.classes().contains("phone-flat"));
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some("save a copy")
        );
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(calls.get(), 1);
        owner.dispose();
    }

    #[test]
    fn automatic_ellipsis_owns_expansion_and_primary_commands_dismiss_it() {
        let _render = render_scope();
        let open = Rc::new(Cell::new(false));
        let read = open.clone();
        let write = open.clone();
        set_application_bar_controller(
            Rc::new(move || read.get()),
            Rc::new(move |value| write.set(value)),
        );
        let called = Rc::new(Cell::new(false));
        let invoked = called.clone();
        let (control, owner) = app_bar(
            vec![action(Icon::Plus, "add", move || invoked.set(true))],
            false,
            vec![],
        )
        .mount_root();
        let more = control
            .find_descendant_of_type_where::<Button>(false, |button| button.has_name("AppBarMore"))
            .unwrap();
        more.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(open.get());
        let add = control
            .find_descendant_of_type_where::<Button>(false, |button| {
                button.has_name("AppBarAction-add")
            })
            .unwrap();
        add.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(called.get());
        assert!(!open.get());
        owner.dispose();
        EXPANSION.with(|slot| slot.borrow_mut().take());
    }

    #[test]
    fn secondary_command_closes_its_own_menu_before_invoking_the_callback() {
        let _render = render_scope();
        let open = Rc::new(Cell::new(true));
        let read = open.clone();
        let write = open.clone();
        set_application_bar_controller(
            Rc::new(move || read.get()),
            Rc::new(move |value| write.set(value)),
        );
        let calls = Rc::new(Cell::new(0));
        let invoked = calls.clone();
        let command_open = open.clone();
        let (control, owner) = app_bar(
            vec![],
            false,
            vec![crate::flat("share text", move || {
                assert!(!command_open.get(), "dismiss precedes surface navigation");
                invoked.set(invoked.get() + 1);
            })],
        )
        .mount_root();
        let command = control
            // ScrollViewer content is mounted into the logical tree even
            // before a window applies the scroll viewer's visual template.
            .get_logical_descendants()
            .filter_map(|child| child.cast::<Button>())
            .find(|button| AutomationProperties::get_name(button).as_deref() == Some("share text"))
            .unwrap();
        command.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(calls.get(), 1);
        assert!(!open.get());
        owner.dispose();
        EXPANSION.with(|slot| slot.borrow_mut().take());
    }

    #[test]
    fn a_command_can_reopen_its_menu_without_the_bubbling_fallback_closing_it() {
        let _render = render_scope();
        let open = Rc::new(Cell::new(true));
        let read = open.clone();
        let write = open.clone();
        set_application_bar_controller(
            Rc::new(move || read.get()),
            Rc::new(move |value| write.set(value)),
        );
        let calls = Rc::new(Cell::new(0));
        let invoked = calls.clone();
        let command_open = open.clone();
        // A primary command exercises a complete native visual event route
        // without needing to apply a ScrollViewer's template first.
        let (control, owner) = app_bar(
            vec![crate::flat("format", move || {
                assert!(!command_open.get(), "dismiss still precedes the callback");
                invoked.set(invoked.get() + 1);
                command_open.set(true);
            })],
            false,
            vec![],
        )
        .mount_root();
        let bubbled = Rc::new(Cell::new(0));
        let observed = bubbled.clone();
        control.add_handler_with(
            Button::click_event(),
            move |_, _| observed.set(observed.get() + 1),
            RoutingStrategies::BUBBLE,
            true,
        );
        let command = control
            .find_descendant_of_type_where::<Button>(false, |button| {
                AutomationProperties::get_name(button).as_deref() == Some("format")
            })
            .unwrap();
        command.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(calls.get(), 1);
        assert_eq!(bubbled.get(), 1, "the handled click reached the bar");
        assert!(open.get(), "the replacement menu remains available");
        owner.dispose();
        EXPANSION.with(|slot| slot.borrow_mut().take());
    }

    #[test]
    fn a_retained_bar_never_dismisses_a_replacement_host() {
        let _render = render_scope();
        let original = Rc::new(Cell::new(false));
        let read = original.clone();
        let write = original.clone();
        set_application_bar_controller(
            Rc::new(move || read.get()),
            Rc::new(move |value| write.set(value)),
        );
        let (control, owner) =
            app_bar(vec![action(Icon::Plus, "add", || {})], false, vec![]).mount_root();
        let replacement = Rc::new(Cell::new(true));
        let read = replacement.clone();
        let write = replacement.clone();
        set_application_bar_controller(
            Rc::new(move || read.get()),
            Rc::new(move |value| write.set(value)),
        );
        let more = control
            .find_descendant_of_type_where::<Button>(false, |button| button.has_name("AppBarMore"))
            .unwrap();
        more.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(original.get());
        let add = control
            .find_descendant_of_type_where::<Button>(false, |button| {
                button.has_name("AppBarAction-add")
            })
            .unwrap();
        add.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(!original.get());
        assert!(replacement.get(), "commands belong to their creating host");
        owner.dispose();
        EXPANSION.with(|slot| slot.borrow_mut().take());
    }

    #[test]
    fn standalone_overflow_can_be_opened_and_invoked_without_a_host() {
        let _render = render_scope();
        EXPANSION.with(|slot| slot.borrow_mut().take());
        let calls = Rc::new(Cell::new(0));
        let invoked = calls.clone();
        let (control, owner) = app_bar(
            vec![
                action(Icon::Plus, "one", || {}),
                action(Icon::Plus, "two", || {}),
                action(Icon::Plus, "three", || {}),
                action(Icon::Plus, "four", || {}),
                action(Icon::Plus, "five", move || invoked.set(invoked.get() + 1)),
            ],
            false,
            vec![],
        )
        .mount_root();
        assert!(control.classes().contains("collapsed"));
        assert!(
            control
                .get_logical_descendants()
                .filter_map(|child| child.cast::<Button>())
                .find(|button| button.has_name("AppBarAction-five"))
                .is_none()
        );
        let more = control
            .find_descendant_of_type_where::<Button>(false, |button| button.has_name("AppBarMore"))
            .unwrap();
        more.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(control.classes().contains("expanded"));
        let fifth = control
            .get_logical_descendants()
            .filter_map(|child| child.cast::<Button>())
            .find(|button| button.has_name("AppBarAction-five"))
            .expect("the fifth command is reachable through the ellipsis");
        assert!(fifth.classes().contains("phone-flat"));
        fifth.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(calls.get(), 1);
        assert!(control.classes().contains("collapsed"));
        owner.dispose();
    }
}
