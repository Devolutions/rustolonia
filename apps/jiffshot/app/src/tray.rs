//! The notification-area icon's model (the menu is built in `ui/tray.rs`);
//! every command runs on the capture side's thread
//! ([`TrayAction::post`]), and the switches show what the capture side last
//! published ([`publish`]).

use crate::vm::{TrayViewModel, TrayViewModelSink};
use jiffshot_workflow::{TrayAction, TrayState};
use rustolonia::Result;
use std::cell::RefCell;

thread_local! {
    static SINK: RefCell<Option<TrayViewModelSink>> = const { RefCell::new(None) };
    static STATE: RefCell<TrayState> = RefCell::new(TrayState::default());
    /// Recent captures, newest first, as file URIs.
    static RECENT: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// The library's newest captures, for the Recent captures submenu. On the
/// UI thread.
pub fn set_recent(uris: Vec<String>) {
    if RECENT.with(|r| *r.borrow() == uris) {
        return;
    }
    RECENT.with(|r| *r.borrow_mut() = uris);
    publish_recent();
}

fn publish_recent() {
    SINK.with(|sink| {
        let Some(sink) = sink.borrow().as_ref().cloned() else { return };
        let _ = sink.clear_recent_captures();
        RECENT.with(|r| {
            for uri in r.borrow().iter() {
                let _ = sink.add_recent_captures(uri.clone());
            }
        });
    });
}

/// Shows `state` in the tray menu. On the UI thread.
pub fn publish(state: TrayState) {
    STATE.with(|s| *s.borrow_mut() = state.clone());
    SINK.with(|sink| {
        let Some(sink) = sink.borrow().as_ref().cloned() else { return };
        let _ = sink.set_copy_to_clipboard(state.copy_to_clipboard);
        let _ = sink.set_open_in_editor(state.open_in_editor);
        let _ = sink.set_include_cursor(state.include_cursor);
        let _ = sink.set_show_widget(state.show_widget);
        let _ = sink.set_run_at_startup(state.run_at_startup);
        let _ = sink.set_time_lapse(state.time_lapse);
        let _ = sink.set_schedule(state.schedule);
        let _ = sink.set_can_repeat(state.can_repeat);
    });
}

/// A switch flipped in the menu: the capture side toggles it (when it is
/// not already that way) and publishes the result.
fn switch(value: bool, now: impl Fn(&TrayState) -> bool, action: TrayAction) -> Result<()> {
    if STATE.with(|s| now(&s.borrow())) != value {
        action.post();
    }
    Ok(())
}

fn run(action: TrayAction) -> Result<()> {
    action.post();
    Ok(())
}

pub struct TrayModel;

impl TrayViewModel for TrayModel {
    fn attach(&mut self, sink: TrayViewModelSink) -> Result<()> {
        SINK.with(|s| *s.borrow_mut() = Some(sink));
        publish(STATE.with(|s| s.borrow().clone()));
        publish_recent();
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        SINK.with(|s| *s.borrow_mut() = None);
        Ok(())
    }

    fn set_copy_to_clipboard(&mut self, value: bool) -> Result<()> {
        switch(value, |s| s.copy_to_clipboard, TrayAction::CopyToClipboard)
    }
    fn set_open_in_editor(&mut self, value: bool) -> Result<()> {
        switch(value, |s| s.open_in_editor, TrayAction::OpenInEditor)
    }
    fn set_include_cursor(&mut self, value: bool) -> Result<()> {
        switch(value, |s| s.include_cursor, TrayAction::IncludeCursor)
    }
    fn set_show_widget(&mut self, value: bool) -> Result<()> {
        switch(value, |s| s.show_widget, TrayAction::ShowWidget)
    }
    fn set_run_at_startup(&mut self, value: bool) -> Result<()> {
        switch(value, |s| s.run_at_startup, TrayAction::RunAtStartup)
    }
    fn set_time_lapse(&mut self, value: bool) -> Result<()> {
        switch(value, |s| s.time_lapse, TrayAction::TimeLapse)
    }
    fn set_schedule(&mut self, value: bool) -> Result<()> {
        switch(value, |s| s.schedule, TrayAction::Schedule)
    }

    fn capture(&mut self) -> Result<()> {
        run(TrayAction::Capture)
    }
    fn capture_delayed(&mut self) -> Result<()> {
        run(TrayAction::CaptureDelayed)
    }
    fn capture_screen(&mut self) -> Result<()> {
        run(TrayAction::CaptureScreen)
    }
    fn repeat_last(&mut self) -> Result<()> {
        run(TrayAction::RepeatLast)
    }
    fn capture_window(&mut self) -> Result<()> {
        run(TrayAction::CaptureWindow)
    }
    fn show_editor(&mut self) -> Result<()> {
        crate::app::post(|app| app.show_editor());
        Ok(())
    }
    fn scrolling(&mut self) -> Result<()> {
        run(TrayAction::Scrolling)
    }
    fn scrolling_horizontal(&mut self) -> Result<()> {
        run(TrayAction::ScrollingHorizontal)
    }
    fn scrolling_both(&mut self) -> Result<()> {
        run(TrayAction::ScrollingBoth)
    }
    fn panoramic(&mut self) -> Result<()> {
        run(TrayAction::Panoramic)
    }
    fn freehand(&mut self) -> Result<()> {
        run(TrayAction::Freehand)
    }
    fn multiple_areas(&mut self) -> Result<()> {
        run(TrayAction::MultipleAreas)
    }
    fn fixed_size(&mut self) -> Result<()> {
        run(TrayAction::FixedSize)
    }
    fn menu_capture(&mut self) -> Result<()> {
        run(TrayAction::MenuCapture)
    }
    fn grab_text(&mut self) -> Result<()> {
        run(TrayAction::GrabText)
    }
    fn step_capture(&mut self) -> Result<()> {
        run(TrayAction::StepCapture)
    }
    fn from_clipboard(&mut self) -> Result<()> {
        run(TrayAction::FromClipboard)
    }
    fn from_file(&mut self) -> Result<()> {
        run(TrayAction::FromFile)
    }
    fn record_video(&mut self) -> Result<()> {
        run(TrayAction::RecordVideo)
    }
    fn open_library_folder(&mut self) -> Result<()> {
        run(TrayAction::OpenLibraryFolder)
    }
    fn preferences(&mut self) -> Result<()> {
        crate::app::post(|app| app.open_prefs("capture"));
        Ok(())
    }
    fn open_recent_capture(&mut self, value: String) -> Result<()> {
        crate::app::post(move |app| {
            app.show_editor();
            app.open_recent(&value);
        });
        Ok(())
    }
    fn exit(&mut self) -> Result<()> {
        // The capture side stops, and its quit hook ends the app.
        run(TrayAction::Exit)
    }
}
