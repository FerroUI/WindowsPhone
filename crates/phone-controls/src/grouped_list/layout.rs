/// The item index of a real group-header row in a vertically scrolling list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListGroup {
    pub key: String,
    pub index: usize,
}

impl ListGroup {
    pub fn new(key: impl Into<String>, index: usize) -> Self {
        Self {
            key: key.into(),
            index,
        }
    }
}

pub(super) struct MeasuredRow {
    pub index: usize,
    pub y: f64,
    pub height: f64,
}

pub(super) fn placement(
    groups: &[ListGroup],
    rows: &[MeasuredRow],
    height: f64,
) -> (Option<String>, f64) {
    let Some(first) = rows.iter().find(|row| row.y + row.height > 0.01) else {
        return (None, 0.0);
    };
    let Some(active) = groups.iter().rposition(|group| {
        group.index < first.index || (group.index == first.index && first.y <= 0.01)
    }) else {
        return (None, 0.0);
    };
    let push = groups
        .get(active + 1)
        .and_then(|next| rows.iter().find(|row| row.index == next.index))
        .map(|next| (next.y - height).clamp(-height, 0.0))
        .unwrap_or(0.0);
    (Some(groups[active].key.clone()), push)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[(usize, f64, f64)]) -> Vec<MeasuredRow> {
        values
            .iter()
            .map(|&(index, y, height)| MeasuredRow { index, y, height })
            .collect()
    }

    #[test]
    fn recycled_header_is_resolved_from_the_first_visible_native_item() {
        let groups = [ListGroup::new("a", 0), ListGroup::new("b", 1000)];
        // A focused, retained container can precede the realized viewport.
        let visible = rows(&[(4, -4000.0, 80.0), (750, -19.0, 91.0), (751, 72.0, 43.0)]);
        assert_eq!(placement(&groups, &visible, 56.0), (Some("a".into()), 0.0));
        assert_eq!(
            placement(&groups, &rows(&[(1001, -12.0, 87.0)]), 56.0),
            (Some("b".into()), 0.0)
        );
    }

    #[test]
    fn actual_next_header_pushes_and_then_replaces_the_current_one() {
        let groups = [ListGroup::new("a", 0), ListGroup::new("b", 4)];
        assert_eq!(
            placement(&groups, &rows(&[(3, -43.0, 73.0), (4, 30.0, 56.0)]), 56.0),
            (Some("a".into()), -26.0)
        );
        assert_eq!(
            placement(&groups, &rows(&[(3, -73.0, 73.0), (4, 0.0, 56.0)]), 56.0),
            (Some("b".into()), 0.0)
        );
        assert_eq!(
            placement(&groups, &rows(&[(4, -12.0, 56.0)]), 56.0),
            (Some("b".into()), 0.0)
        );
    }

    #[test]
    fn list_padding_empty_results_and_unreached_first_header_have_no_overlay() {
        let groups = [ListGroup::new("a", 0)];
        assert_eq!(
            placement(&groups, &rows(&[(0, 16.0, 56.0)]), 56.0),
            (None, 0.0)
        );
        assert_eq!(placement(&groups, &[], 56.0), (None, 0.0));
        assert_eq!(
            placement(&[], &rows(&[(0, -16.0, 56.0)]), 56.0),
            (None, 0.0)
        );
    }

    #[test]
    fn final_group_header_below_the_viewport_top_keeps_the_previous_group_active() {
        let groups = [ListGroup::new("a", 0), ListGroup::new("z", 100)];
        assert_eq!(
            placement(
                &groups,
                &rows(&[(99, -14.0, 240.0), (100, 226.0, 56.0)]),
                56.0
            ),
            (Some("a".into()), 0.0)
        );
        assert_eq!(
            placement(&groups, &rows(&[(100, 26.0, 56.0)]), 56.0),
            (Some("a".into()), -30.0)
        );
    }
}
