//! Date formatting preferences; calendar data always retains ISO dates.
use super::update;
use ferroui_view::prelude::*;
use phone_controls::{flat, toggle};
use phone_domain::DateOrder;
use phone_sdk::common::{muted, text};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let (year, month, day) = phone_domain::date_parts(vm.local_now());
    let mut countries = stack_panel().spacing(5.0);
    for (name, order, monday) in [
        ("United Kingdom", DateOrder::DayMonthYear, true),
        ("United States", DateOrder::MonthDayYear, false),
        ("Poland", DateOrder::DayMonthYear, true),
    ] {
        let model = vm.clone();
        countries = countries.child(flat(
            &format!(
                "{} {name}",
                if settings.region == name {
                    "●"
                } else {
                    "○"
                }
            ),
            move || {
                update(&model, |s| {
                    s.region = name.into();
                    s.date_order = order;
                    s.first_day_monday = monday;
                })
            },
        ));
    }
    let mut orders = stack_panel().spacing(5.0);
    for order in [
        DateOrder::DayMonthYear,
        DateOrder::MonthDayYear,
        DateOrder::YearMonthDay,
    ] {
        let model = vm.clone();
        orders = orders.child(flat(
            &format!(
                "{} {}",
                if settings.date_order == order {
                    "●"
                } else {
                    "○"
                },
                order.label()
            ),
            move || update(&model, |s| s.date_order = order),
        ));
    }
    let first_day = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {muted("Country/Region",18.0)}{countries}
        {muted("Regional format",18.0)}{text(&settings.date_order.format(year,month,day),30.0)}
        {orders}
        {toggle("Start the week on Monday",settings.first_day_monday,move |enabled|update(&first_day,|s|s.first_day_monday=enabled))}
        {muted("Formats local calendar and date displays. The interface language is English.",16.0)}
    }}
}
