use super::ComponentContext;
use rustolonia::{
    use_context, CheckBox, ComboBox, Control, Downcast, El, Error, Grid, Label, Result, TextBlock,
    TextBox, Thickness, VerticalAlignment,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, TryLockError};

const LABEL_LIKE_MARKER: &str = "form-grid-label-like";

#[derive(Clone, Copy, Debug)]
pub struct FormGridOptions {
    pub bottom_margin: f64,
    pub label_right_margin: f64,
    pub preceded_label_left_margin: f64,
    pub treat_empty_cells_as_controls: bool,
    pub apply_sample_styles: bool,
}

impl Default for FormGridOptions {
    fn default() -> Self {
        Self {
            bottom_margin: 5.0,
            label_right_margin: 5.0,
            preceded_label_left_margin: 4.0,
            treat_empty_cells_as_controls: false,
            apply_sample_styles: true,
        }
    }
}

pub fn form_grid() -> El<Grid> {
    Grid::el()
        .row_spacing(0.0)
        .column_spacing(0.0)
        .vertical_alignment(VerticalAlignment::Top)
}

pub fn finish_form_grid(grid: El<Grid>) -> El<Grid> {
    finish_form_grid_with_options(grid, FormGridOptions::default())
}

pub fn finish_form_grid_with_options(grid: El<Grid>, options: FormGridOptions) -> El<Grid> {
    grid.with_op(move |scope, grid| {
        let context = use_context::<ComponentContext>()
            .ok_or_else(|| Error::Binding("FormGrid requires ComponentContext".into()))?;
        let state = Arc::new(Mutex::new(FormGridState::default()));
        state
            .lock()
            .map_err(|_| Error::Binding("FormGrid state lock poisoned".into()))?
            .refresh(grid, &context, options)?;
        let pending = Arc::new(AtomicBool::new(false));
        let children_grid = grid.clone();
        let children_state = state.clone();
        let children_pending = pending.clone();
        let subscription = grid.subscribe_children_changed(move |_| {
            update_state(
                &children_state,
                &children_grid,
                options,
                &children_pending,
                |state| state.remove_missing(&children_grid, options),
            );
        })?;
        scope.retain_subscription(subscription);
        let layout_grid = grid.clone();
        let subscription = grid.subscribe_layout_updated(move |_| {
            update_state(&state, &layout_grid, options, &pending, |state| {
                state.refresh(&layout_grid, &context, options)
            });
        })?;
        scope.retain_subscription(subscription);
        Ok(())
    })
}

fn update_state(
    state: &Mutex<FormGridState>,
    grid: &Grid,
    options: FormGridOptions,
    pending: &AtomicBool,
    update: impl FnOnce(&mut FormGridState) -> Result<()>,
) {
    let mut state = match state.try_lock() {
        Ok(state) => state,
        Err(TryLockError::WouldBlock) => {
            pending.store(true, Ordering::Release);
            return;
        }
        Err(TryLockError::Poisoned(_)) => {
            eprintln!("FormGrid update failed: state lock poisoned");
            return;
        }
    };
    let result = update(&mut state).and_then(|()| {
        while pending.swap(false, Ordering::AcqRel) {
            state.remove_missing(grid, options)?;
        }
        Ok(())
    });
    if let Err(error) = result {
        eprintln!("FormGrid update failed: {error}");
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Classes {
    item: bool,
    label: bool,
    last_row: bool,
    last_column: bool,
    preceded: bool,
    out_of_bounds: bool,
}

impl Classes {
    fn read(control: &Control) -> Result<Self> {
        let classes = control.classes()?;
        Ok(Self {
            item: classes.contains("form-grid-item")?,
            label: classes.contains("label-like-item")?,
            last_row: classes.contains("last-row-item")?,
            last_column: classes.contains("last-column-item")?,
            preceded: classes.contains("preceded-by-control")?,
            out_of_bounds: classes.contains("out-of-bounds")?,
        })
    }

    fn apply(self, control: &Control) -> Result<()> {
        let classes = control.classes()?;
        for (name, enabled) in [
            ("form-grid-item", self.item),
            ("label-like-item", self.label),
            ("last-row-item", self.last_row),
            ("last-column-item", self.last_column),
            ("preceded-by-control", self.preceded),
            ("out-of-bounds", self.out_of_bounds),
        ] {
            if classes.contains(name)? != enabled {
                if enabled {
                    classes.add(name)?;
                } else {
                    classes.remove_value(name)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct Item {
    row: usize,
    column: usize,
    end_row: usize,
    end_column: usize,
    visible: bool,
    label: bool,
    classes: Classes,
}

fn classify(
    items: &[Item],
    row_definitions: usize,
    column_definitions: usize,
    treat_empty_cells_as_controls: bool,
) -> Result<Vec<Classes>> {
    let rows = items
        .iter()
        .filter(|item| item.visible)
        .map(|item| item.end_row)
        .max()
        .unwrap_or(0)
        .min(row_definitions.max(1));
    let columns = items
        .iter()
        .filter(|item| item.visible)
        .map(|item| item.end_column)
        .max()
        .unwrap_or(0)
        .min(column_definitions.max(1));
    let cell_count = rows
        .checked_mul(columns)
        .ok_or_else(|| Error::Binding("FormGrid dimensions are too large".into()))?;
    let mut cells = Vec::new();
    cells
        .try_reserve_exact(cell_count)
        .map_err(|error| Error::Binding(format!("FormGrid cannot allocate its cells: {error}")))?;
    cells.resize(cell_count, None::<usize>);
    let mut result: Vec<_> = items.iter().map(|item| item.classes).collect();

    for (index, item) in items.iter().enumerate() {
        if !item.visible {
            result[index] = Classes::default();
            continue;
        }
        result[index].out_of_bounds = item.end_row > rows || item.end_column > columns;
        for row in item.row..item.end_row.min(rows) {
            for column in item.column..item.end_column.min(columns) {
                cells[row * columns + column].get_or_insert(index);
            }
        }
    }

    for row in 0..rows {
        for column in 0..columns {
            let Some(index) = cells[row * columns + column] else {
                continue;
            };
            let previous = column
                .checked_sub(1)
                .and_then(|previous| cells[row * columns + previous]);
            if previous == Some(index) {
                continue;
            }
            let item = items[index];
            let classes = &mut result[index];
            classes.item = true;
            classes.label = item.label;
            classes.last_row = item.end_row == rows;
            classes.last_column = item.end_column == columns;
            classes.preceded = match previous {
                Some(previous) => !items[previous].label,
                None => column > 0 && treat_empty_cells_as_controls,
            };
        }
    }
    Ok(result)
}

fn position(value: i32, name: &str) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| Error::Binding(format!("FormGrid {name} must not be negative")))
}

fn item(control: &Control, grid_visible: bool) -> Result<Item> {
    let row = position(Grid::row(control)?, "row")?;
    let column = position(Grid::column(control)?, "column")?;
    let row_span = position(Grid::row_span(control)?, "row span")?;
    let column_span = position(Grid::column_span(control)?, "column span")?;
    if row_span == 0 || column_span == 0 {
        return Err(Error::Binding("FormGrid spans must be positive".into()));
    }
    let end_row = row
        .checked_add(row_span)
        .ok_or_else(|| Error::Binding("FormGrid row span is too large".into()))?;
    let end_column = column
        .checked_add(column_span)
        .ok_or_else(|| Error::Binding("FormGrid column span is too large".into()))?;
    let label = <Control as Downcast<Label>>::is_instance(control)
        || <Control as Downcast<CheckBox>>::is_instance(control)
        || <Control as Downcast<TextBlock>>::is_instance(control)
        || control.classes()?.contains(LABEL_LIKE_MARKER)?;
    Ok(Item {
        row,
        column,
        end_row,
        end_column,
        visible: grid_visible && control.is_visible()?,
        label,
        classes: Classes::read(control)?,
    })
}

#[derive(Default)]
struct FormGridState {
    children: HashMap<i64, TrackedChild>,
}

impl FormGridState {
    fn refresh(
        &mut self,
        grid: &Grid,
        context: &ComponentContext,
        options: FormGridOptions,
    ) -> Result<()> {
        let children = grid.children()?;
        let mut ids = HashSet::new();
        let mut current = Vec::new();
        let mut items = Vec::new();
        let grid_visible = grid.is_effectively_visible()?;
        for index in 0..children.len()? {
            let control = children.get(index)?;
            let id = control.object_id()?;
            if !ids.insert(id) {
                return Err(Error::Binding(
                    "FormGrid cannot contain the same control twice".into(),
                ));
            }
            if !self.children.contains_key(&id) {
                self.children
                    .insert(id, TrackedChild::new(control.clone(), options)?);
            }
            items.push(item(&control, grid_visible)?);
            current.push(id);
        }

        self.remove_absent(&ids, options)?;

        let classes = classify(
            &items,
            grid.row_definitions()?.len(),
            grid.column_definitions()?.len(),
            options.treat_empty_cells_as_controls,
        )?;
        for (id, classes) in current.into_iter().zip(classes) {
            let child = self
                .children
                .get_mut(&id)
                .ok_or_else(|| Error::Binding("FormGrid lost a tracked child".into()))?;
            classes.apply(&child.control)?;
            child.apply_style(classes, context, options)?;
        }
        Ok(())
    }

    fn remove_missing(&mut self, grid: &Grid, options: FormGridOptions) -> Result<()> {
        let children = grid.children()?;
        let mut ids = HashSet::new();
        for index in 0..children.len()? {
            ids.insert(children.get(index)?.object_id()?);
        }
        self.remove_absent(&ids, options)
    }

    fn remove_absent(&mut self, ids: &HashSet<i64>, options: FormGridOptions) -> Result<()> {
        let removed: Vec<_> = self
            .children
            .keys()
            .filter(|id| !ids.contains(id))
            .copied()
            .collect();
        for id in removed {
            if let Some(child) = self.children.get_mut(&id) {
                Classes::default().apply(&child.control)?;
                child.restore(options)?;
            }
            self.children.remove(&id);
        }
        Ok(())
    }
}

struct TrackedChild {
    control: Control,
    original_margin: Thickness,
    managed_margin: bool,
    style: ChildStyle,
}

impl TrackedChild {
    fn new(control: Control, options: FormGridOptions) -> Result<Self> {
        let style = if options.apply_sample_styles {
            ChildStyle::capture(&control)?
        } else {
            ChildStyle::Other
        };
        Ok(Self {
            original_margin: control.margin()?,
            control,
            managed_margin: false,
            style,
        })
    }

    fn apply_style(
        &mut self,
        classes: Classes,
        context: &ComponentContext,
        options: FormGridOptions,
    ) -> Result<()> {
        if !options.apply_sample_styles {
            return Ok(());
        }
        self.style.apply(&self.control, context.compact_height)?;
        if classes.item {
            let left = if classes.label && classes.preceded {
                options.preceded_label_left_margin
            } else {
                0.0
            };
            let right = if classes.last_column {
                0.0
            } else if classes.label {
                if classes.preceded {
                    options.label_right_margin
                } else {
                    5.0
                }
            } else {
                context.right_margin
            };
            let bottom = if classes.last_row {
                0.0
            } else {
                options.bottom_margin
            };
            set_margin(&self.control, Thickness::new(left, 0.0, right, bottom))?;
            self.managed_margin = true;
        } else if self.managed_margin {
            set_margin(&self.control, self.original_margin)?;
            self.managed_margin = false;
        }
        Ok(())
    }

    fn restore(&mut self, options: FormGridOptions) -> Result<()> {
        if options.apply_sample_styles {
            if self.managed_margin {
                set_margin(&self.control, self.original_margin)?;
                self.managed_margin = false;
            }
            self.style.restore(&self.control)?;
        }
        Ok(())
    }
}

enum ChildStyle {
    Input {
        height: f64,
        min_height: f64,
        alignment: VerticalAlignment,
    },
    Text {
        text: TextBlock,
        padding: Thickness,
        max_height: f64,
    },
    Label {
        label: Label,
        padding: Thickness,
        max_height: f64,
        alignment: VerticalAlignment,
        content_alignment: VerticalAlignment,
    },
    Other,
}

impl ChildStyle {
    fn capture(control: &Control) -> Result<Self> {
        if <Control as Downcast<ComboBox>>::is_instance(control)
            || <Control as Downcast<TextBox>>::is_instance(control)
        {
            return Ok(Self::Input {
                height: control.height()?,
                min_height: control.min_height()?,
                alignment: control.vertical_alignment()?,
            });
        }
        if let Some(label) = <Control as Downcast<Label>>::downcast(control) {
            return Ok(Self::Label {
                padding: label.padding()?,
                max_height: control.max_height()?,
                alignment: control.vertical_alignment()?,
                content_alignment: label.vertical_content_alignment()?,
                label,
            });
        }
        if let Some(text) = <Control as Downcast<TextBlock>>::downcast(control) {
            return Ok(Self::Text {
                padding: text.padding()?,
                max_height: control.max_height()?,
                text,
            });
        }
        Ok(Self::Other)
    }

    fn apply(&self, control: &Control, compact_height: f64) -> Result<()> {
        match self {
            Self::Input { .. } => {
                set_height(control, compact_height)?;
                if control.min_height()? != compact_height {
                    control.set_min_height(compact_height)?;
                }
                set_alignment(control, VerticalAlignment::Top)?;
            }
            Self::Text { text, .. } => {
                set_max_height(control, compact_height)?;
                if text.padding()? != Thickness::uniform(3.0) {
                    text.set_padding(Thickness::uniform(3.0))?;
                }
            }
            Self::Label { label, .. } => {
                set_max_height(control, compact_height)?;
                set_alignment(control, VerticalAlignment::Top)?;
                if label.padding()? != Thickness::symmetric(3.0, 0.0) {
                    label.set_padding(Thickness::symmetric(3.0, 0.0))?;
                }
                if label.vertical_content_alignment()? != VerticalAlignment::Center {
                    label.set_vertical_content_alignment(VerticalAlignment::Center)?;
                }
            }
            Self::Other => {}
        }
        Ok(())
    }

    fn restore(&self, control: &Control) -> Result<()> {
        match self {
            Self::Input {
                height,
                min_height,
                alignment,
            } => {
                set_height(control, *height)?;
                if control.min_height()? != *min_height {
                    control.set_min_height(*min_height)?;
                }
                set_alignment(control, *alignment)?;
            }
            Self::Text {
                text,
                padding,
                max_height,
            } => {
                set_max_height(control, *max_height)?;
                if text.padding()? != *padding {
                    text.set_padding(*padding)?;
                }
            }
            Self::Label {
                label,
                padding,
                max_height,
                alignment,
                content_alignment,
            } => {
                set_max_height(control, *max_height)?;
                set_alignment(control, *alignment)?;
                if label.padding()? != *padding {
                    label.set_padding(*padding)?;
                }
                if label.vertical_content_alignment()? != *content_alignment {
                    label.set_vertical_content_alignment(*content_alignment)?;
                }
            }
            Self::Other => {}
        }
        Ok(())
    }
}

fn set_margin(control: &Control, value: Thickness) -> Result<()> {
    if control.margin()? != value {
        control.set_margin(value)?;
    }
    Ok(())
}

fn same_float(left: f64, right: f64) -> bool {
    left == right || (left.is_nan() && right.is_nan())
}

fn set_height(control: &Control, value: f64) -> Result<()> {
    if !same_float(control.height()?, value) {
        control.set_height(value)?;
    }
    Ok(())
}

fn set_max_height(control: &Control, value: f64) -> Result<()> {
    if !same_float(control.max_height()?, value) {
        control.set_max_height(value)?;
    }
    Ok(())
}

fn set_alignment(control: &Control, value: VerticalAlignment) -> Result<()> {
    if control.vertical_alignment()? != value {
        control.set_vertical_alignment(value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(row: usize, column: usize, rows: usize, columns: usize, label: bool) -> Item {
        Item {
            row,
            column,
            end_row: row + rows,
            end_column: column + columns,
            visible: true,
            label,
            classes: Classes::default(),
        }
    }

    #[test]
    fn hidden_rows_are_not_renumbered_and_hidden_classes_are_cleared() {
        let mut hidden = cell(1, 0, 1, 1, false);
        hidden.visible = false;
        hidden.classes = Classes {
            item: true,
            out_of_bounds: true,
            ..Classes::default()
        };
        let result = classify(
            &[cell(0, 0, 1, 1, false), hidden, cell(2, 0, 1, 1, true)],
            3,
            1,
            false,
        )
        .unwrap();
        assert!(result[0].item && !result[0].last_row);
        assert_eq!(result[1], Classes::default());
        assert!(result[2].last_row && result[2].last_column && result[2].label);
    }

    #[test]
    fn spans_use_first_occupant_and_keep_shadowed_classes() {
        let mut shadowed = cell(0, 0, 1, 1, true);
        shadowed.classes = Classes {
            item: true,
            last_row: true,
            out_of_bounds: true,
            ..Classes::default()
        };
        let result = classify(
            &[cell(0, 0, 1, 2, false), cell(0, 1, 1, 2, true), shadowed],
            1,
            3,
            false,
        )
        .unwrap();
        assert!(result[0].item && !result[0].last_column);
        assert!(result[1].item && result[1].preceded && result[1].last_column);
        assert!(result[2].item && result[2].last_row && !result[2].label);
        assert!(!result[2].out_of_bounds);
    }

    #[test]
    fn row_spans_can_be_reclassified_by_a_later_rows_neighbor() {
        let result = classify(
            &[
                cell(0, 0, 1, 1, false),
                cell(1, 0, 1, 1, true),
                cell(0, 1, 2, 1, false),
            ],
            2,
            2,
            false,
        )
        .unwrap();
        assert!(result[2].item && result[2].last_row && result[2].last_column);
        assert!(!result[2].preceded);
    }

    #[test]
    fn empty_cells_are_only_predecessors_when_enabled() {
        let items = [cell(0, 2, 1, 1, true)];
        assert!(!classify(&items, 1, 3, false).unwrap()[0].preceded);
        assert!(classify(&items, 1, 3, true).unwrap()[0].preceded);
    }

    #[test]
    fn implicit_tracks_and_partial_spans_report_out_of_bounds() {
        let result = classify(
            &[cell(0, 0, 1, 2, false), cell(2, 0, 1, 1, true)],
            0,
            0,
            false,
        )
        .unwrap();
        assert!(result[0].item && result[0].out_of_bounds);
        assert!(!result[0].last_column);
        assert!(!result[1].item && result[1].out_of_bounds);
    }
}
