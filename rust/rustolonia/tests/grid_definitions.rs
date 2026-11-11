//! The safe surface for `Grid`'s track definitions (`GridTracks`, `GridTrack`, `GridLength`,
//! and `tracks!`).
//!
//! Managed `ColumnDefinitions`/`RowDefinitions` round-tripping across a live host is covered by
//! `nativeaot_object_model`. These tests verify the typed Rust API: setters and `El<Grid>`
//! builders accept strings, `GridTracks`, `[GridTrack; N]`, `[GridLength; N]`, or `tracks!`,
//! while getters return `Result<GridTracks>`.

use rustolonia::{tracks, El, Error, Grid, GridLength, GridTrack, GridTracks, Result, Signal};

#[allow(dead_code)]
fn definitions_are_written_from_strings_arrays_and_tracks_macro(grid: &Grid) -> Result<()> {
    let owned = String::from("Auto,*");
    grid.set_column_definitions("*,Auto,120")?;
    grid.set_row_definitions(String::from("Auto,2*"))?;
    grid.set_column_definitions(&owned)?;
    grid.set_column_definitions(tracks![*, Auto, 120])?;
    grid.set_row_definitions([GridTrack::auto(), GridTrack::star(2.0)])?;
    grid.set_column_definitions([
        GridLength::Star(1.0),
        GridLength::Auto,
        GridLength::Px(120.0),
    ])?;
    // An empty list clears the tracks.
    grid.set_row_definitions("")?;
    grid.set_row_definitions(GridTracks::new())
}

#[allow(dead_code)]
fn definitions_are_read_back_as_typed_grid_tracks(grid: &Grid) -> Result<(GridTracks, GridTracks)> {
    Ok((grid.column_definitions()?, grid.row_definitions()?))
}

#[allow(dead_code)]
fn definitions_chain_on_declarative_el_builders(reactive_rows: Signal<GridTracks>) -> El<Grid> {
    Grid::el()
        .column_definitions(tracks![*, Auto, 120])
        .row_definitions(reactive_rows)
}

#[test]
fn grid_getters_return_typed_grid_tracks() {
    fn assert_tracks_shaped<F: Fn(&Grid) -> Result<(GridTracks, GridTracks)>>(_: F) {}
    assert_tracks_shaped(definitions_are_read_back_as_typed_grid_tracks);
}

#[test]
fn grid_tracks_parse_format_and_match_tracks_macro() {
    let built = tracks![*, Auto@Labels, 120, 2.5*];
    assert_eq!(built.len(), 4);
    assert!(!built.is_empty());
    assert_eq!(
        built.as_slice(),
        &[
            GridTrack::star(1.0),
            GridTrack::auto().shared_group("Labels"),
            GridTrack::px(120.0),
            GridTrack::star(2.5),
        ]
    );
    assert_eq!(built.to_string(), "*,Auto@Labels,120,2.5*");
    assert_eq!(
        "1*, Auto@Labels, 120, 2.5*".parse::<GridTracks>().unwrap(),
        built
    );
    assert_eq!(
        "1*,Auto,120".parse::<GridTracks>().unwrap().to_string(),
        "*,Auto,120"
    );
    assert_eq!(tracks!["*,Auto,120"].to_string(), "*,Auto,120");

    let empty: GridTracks = "".parse().unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty, tracks![]);
    assert_eq!(empty.to_string(), "");

    assert!(matches!(
        "-5".parse::<GridTracks>(),
        Err(Error::InvalidValue(_))
    ));
    assert!(matches!(
        "-1*".parse::<GridTracks>(),
        Err(Error::InvalidValue(_))
    ));
    assert!(matches!(
        "Auto@".parse::<GridTracks>(),
        Err(Error::InvalidValue(_))
    ));
    assert!(matches!(
        "Auto@A@B".parse::<GridTracks>(),
        Err(Error::InvalidValue(_))
    ));
}
