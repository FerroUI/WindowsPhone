use ferroui_base::media::TranslateTransform;
use ferroui_base::{Ref, Vector};
use ferroui_controls::{Control, ScrollViewer, StackPanel};
use ferroui_view::prelude::*;
use std::cell::RefCell;

pub(super) const HEADER_HEIGHT: f64 = 43.0;

pub(crate) struct AppListState {
    pub letter: Signal<Option<char>>,
    pub transform: Ref<TranslateTransform>,
    headers: RefCell<Vec<(char, f64)>>,
    signature: RefCell<String>,
}

impl Default for AppListState {
    fn default() -> Self {
        Self {
            letter: signal(None),
            transform: TranslateTransform::new(),
            headers: RefCell::new(Vec::new()),
            signature: RefCell::new(String::new()),
        }
    }
}

impl AppListState {
    pub(crate) fn reset(&self) {
        self.letter.set(None);
        self.transform.set_y(0.0);
        self.headers.borrow_mut().clear();
        self.signature.borrow_mut().clear();
    }

    pub(super) fn prepare(&self, signature: String) {
        if *self.signature.borrow() != signature {
            self.reset();
            *self.signature.borrow_mut() = signature;
        }
    }

    pub(super) fn update(&self, scroll: &ScrollViewer, layout_changed: bool) {
        if layout_changed || self.headers.borrow().is_empty() {
            self.refresh(scroll);
        }
        let (letter, y) = placement(&self.headers.borrow(), scroll.offset().y);
        if self.letter.get_untracked() != letter {
            self.letter.set(letter);
        }
        if (self.transform.y() - y).abs() > 0.001 {
            self.transform.set_y(y);
        }
    }

    pub(super) fn jump(&self, scroll: &ScrollViewer, letter: char) {
        self.refresh(scroll);
        let y = self
            .headers
            .borrow()
            .iter()
            .find(|(key, _)| *key == letter)
            .map(|(_, y)| *y);
        if let Some(y) = y {
            scroll.set_offset(Vector::new(0.0, y));
            self.update(scroll, false);
        }
    }

    fn refresh(&self, scroll: &ScrollViewer) {
        let rows = scroll
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|control| control.cast::<StackPanel>());
        let mut headers = self.headers.borrow_mut();
        headers.clear();
        if let Some(rows) = rows {
            for child in rows.children().snapshot().iter() {
                if let Some(letter) = child.name().and_then(|name| {
                    name.strip_prefix("AppGroup-")
                        .and_then(|tail| tail.chars().next())
                }) {
                    headers.push((letter, child.bounds().y));
                }
            }
        }
    }
}

fn placement(headers: &[(char, f64)], offset: f64) -> (Option<char>, f64) {
    let active = headers.iter().rposition(|(_, y)| *y <= offset);
    let Some(index) = active else {
        return (None, 0.0);
    };
    let push = headers
        .get(index + 1)
        .map(|(_, y)| (y - offset - HEADER_HEIGHT).min(0.0))
        .unwrap_or(0.0);
    (Some(headers[index].0), push)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_native_header_pushes_the_current_header_offscreen() {
        let headers = [('a', 10.0), ('b', 90.0), ('c', 280.0)];
        assert_eq!(placement(&headers, 0.0), (None, 0.0));
        assert_eq!(placement(&headers, 10.0), (Some('a'), 0.0));
        assert_eq!(placement(&headers, 80.0), (Some('a'), -33.0));
        assert_eq!(placement(&headers, 90.0), (Some('b'), 0.0));
        assert_eq!(placement(&headers, 500.0), (Some('c'), 0.0));
        assert_eq!(placement(&[], 500.0), (None, 0.0));
    }
}
