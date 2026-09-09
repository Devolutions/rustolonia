//! The safe surface for `Grid`'s track definitions.
//!
//! Parsing is owned by the managed `ColumnDefinitions`/`RowDefinitions`, so a real round trip
//! needs a live host and is covered by `nativeaot_object_model`. These tests pin the shape of
//! the safe API itself: a definition list is a plain string in Rust, written from anything
//! string-like and read back as an owned `String` that is never optional.

use avalonia::{Grid, Result};

#[allow(dead_code)]
fn definitions_are_written_from_anything_string_like(grid: &Grid) -> Result<()> {
    let owned = String::from("Auto,*");
    grid.set_column_definitions("*,Auto,120")?;
    grid.set_row_definitions(String::from("Auto,2*"))?;
    grid.set_column_definitions(&owned)?;
    // An empty list clears the tracks; there is no `None` to pass.
    grid.set_row_definitions("")
}

#[allow(dead_code)]
fn definitions_are_read_back_as_owned_strings(grid: &Grid) -> Result<(String, String)> {
    Ok((grid.get_column_definitions()?, grid.get_row_definitions()?))
}

#[allow(dead_code)]
fn definitions_chain_as_builders(grid: Grid) -> Result<Grid> {
    grid.column_definitions("*,Auto,120")?
        .row_definitions("Auto,2*")
}

#[test]
fn a_definition_list_is_a_string_rather_than_a_projected_collection() {
    // The generated safe getters return `String`, not a collection handle, so the whole
    // feature is expressible without any definition object crossing the ABI. If that ever
    // changed, the signatures above would stop compiling.
    fn assert_string_shaped<F: Fn(&Grid) -> Result<(String, String)>>(_: F) {}
    assert_string_shaped(definitions_are_read_back_as_owned_strings);
}
