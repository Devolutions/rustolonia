//! NeoHtop Avalonia-in-Rust desktop application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use regex::Regex;
use sysinfo::{Disks, Networks, System};

use crate::filter::Filters;
use crate::monitoring::{PendingKill, ProcessInfo, ProcessMonitor, SystemMonitor, SystemStats};
use crate::sort::{process_key, sort_label, sort_visible_indices, SortColumn};

pub use avalonia::{
    App, AppScope, CancellationToken, ClipboardData, Error, MapKey, RangeBatch, RangeRequest,
    Result,
};

mod filter;
mod format;
mod icon;
mod monitoring;
mod reconcile;
mod refresh;
mod sort;

pub mod value_converter {
    pub use avalonia::value_converter::ValueConverterDispatch;
}

pub mod view_model {
    pub use avalonia::view_model::{
        BatchCompletion, DynamicViewModel, ViewModelBatch, ViewModelSink,
    };
}

#[path = "../../generated_view_models.rs"]
mod generated_view_models;

use generated_view_models::{
    mount_main_window, CpuCoreViewModel, CpuCoreViewModelSink, MainViewModel, MainViewModelSink,
    ProcessRowViewModel, ProcessRowViewModelSink,
};

const REFRESH_RATES: [u64; 6] = [1000, 2000, 3000, 5000, 10_000, 30_000];

fn refresh_label(ms: u64) -> &'static str {
    match ms {
        1000 => "1s",
        2000 => "2s",
        3000 => "3s",
        5000 => "5s",
        10_000 => "10s",
        30_000 => "30s",
        _ => "3s",
    }
}

#[derive(Clone, Copy)]
struct ColumnVisibility {
    pid: bool,
    status: bool,
    user: bool,
    cpu: bool,
    ram: bool,
    virt: bool,
    disk: bool,
    run_time: bool,
    command: bool,
    ppid: bool,
    root: bool,
    environ: bool,
    session: bool,
    start_time: bool,
}

impl Default for ColumnVisibility {
    fn default() -> Self {
        Self {
            pid: true,
            status: true,
            user: true,
            cpu: true,
            ram: true,
            virt: true,
            disk: true,
            run_time: true,
            command: false,
            ppid: false,
            root: false,
            environ: false,
            session: false,
            start_time: false,
        }
    }
}

#[derive(Default)]
struct Shared {
    sink: Option<MainViewModelSink>,
    scope: Option<AppScope>,
    window: Option<avalonia::Window>,
}

struct MonitorState {
    sys: System,
    disks: Disks,
    networks: Networks,
    process_monitor: ProcessMonitor,
    system_monitor: SystemMonitor,
    processes: Vec<ProcessInfo>,
    stats: SystemStats,
    visible: Vec<usize>,
    published_keys: Vec<String>,
    published_rows: HashMap<String, RowModel>,
    search_text: String,
    filters: Filters,
    pinned: HashSet<String>,
    selected_index: i64,
    selected_key: String,
    selected_pid: i64,
    sort_column: SortColumn,
    sort_descending: bool,
    is_frozen: bool,
    is_dark_theme: bool,
    show_kill_confirm: bool,
    pending_kill: Option<PendingKill>,
    kill_confirm_message: String,
    show_details: bool,
    detail_pid: u32,
    show_filters: bool,
    show_columns: bool,
    show_search_help: bool,
    columns: ColumnVisibility,
    refresh_ms: u64,
    batch_generation: i64,
    regex_cache: Mutex<HashMap<String, Option<Regex>>>,
}

impl MonitorState {
    fn new() -> Self {
        let sys = System::new_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        Self::with_monitors(sys, disks, networks)
    }

    fn with_monitors(sys: System, disks: Disks, networks: Networks) -> Self {
        Self {
            system_monitor: SystemMonitor::new(&networks),
            process_monitor: ProcessMonitor::new(),
            sys,
            disks,
            networks,
            processes: Vec::new(),
            stats: SystemStats::default(),
            visible: Vec::new(),
            published_keys: Vec::new(),
            published_rows: HashMap::new(),
            search_text: String::new(),
            filters: Filters::default(),
            pinned: HashSet::new(),
            selected_index: -1,
            selected_key: String::new(),
            selected_pid: 0,
            sort_column: SortColumn::CpuUsage,
            sort_descending: true,
            is_frozen: false,
            is_dark_theme: true,
            show_kill_confirm: false,
            pending_kill: None,
            kill_confirm_message: String::new(),
            show_details: false,
            detail_pid: 0,
            show_filters: false,
            show_columns: false,
            show_search_help: false,
            columns: ColumnVisibility::default(),
            refresh_ms: 3000,
            batch_generation: 1,
            regex_cache: Mutex::new(HashMap::new()),
        }
    }

    fn collect(&mut self) -> std::result::Result<(), String> {
        self.sys.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);
        self.processes = self.process_monitor.collect_processes(&self.sys)?;
        self.stats = self
            .system_monitor
            .collect_stats(&self.sys, &self.networks, &self.disks);
        self.rebuild_visible();
        Ok(())
    }

    fn rebuild_visible(&mut self) {
        self.visible = filter::filter_processes(
            &self.processes,
            &self.search_text,
            &self.filters,
            &self.regex_cache,
        );
        sort_visible_indices(
            &self.processes,
            &mut self.visible,
            self.sort_column,
            self.sort_descending,
            &self.pinned,
        );
        self.restore_selection();
    }

    fn restore_selection(&mut self) {
        if let Some(position) = self
            .visible
            .iter()
            .position(|&index| process_key(&self.processes[index]) == self.selected_key)
        {
            self.selected_index = position as i64;
            self.selected_pid = self.processes[self.visible[position]].pid as i64;
        } else {
            self.selected_index = -1;
        }
    }

    fn next_batch_generation(&mut self) -> i64 {
        self.batch_generation = self.batch_generation.saturating_add(1);
        self.batch_generation
    }

    fn selected_process(&self) -> Option<&ProcessInfo> {
        self.visible
            .get(usize::try_from(self.selected_index).ok()?)
            .and_then(|&index| self.processes.get(index))
    }

    fn process_by_pid(&self, pid: u32) -> Option<&ProcessInfo> {
        self.processes.iter().find(|process| process.pid == pid)
    }

    fn visible_process_by_key(&self, key: &str) -> Option<&ProcessInfo> {
        self.visible
            .iter()
            .map(|&index| &self.processes[index])
            .find(|process| process_key(process) == key)
    }
}

struct Model {
    state: Arc<Mutex<MonitorState>>,
    shared: Arc<Mutex<Shared>>,
    sink: Option<MainViewModelSink>,
    refresh: Option<refresh::RefreshWorker>,
}

impl Model {
    fn new(state: Arc<Mutex<MonitorState>>, shared: Arc<Mutex<Shared>>) -> Self {
        Self {
            state,
            shared,
            sink: None,
            refresh: None,
        }
    }
}

impl MainViewModel for Model {
    fn attach(&mut self, sink: MainViewModelSink) -> Result<()> {
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let _ = state.collect();
            state.published_keys.clear();
            state.published_rows.clear();
            publish_snapshot(&mut state, &sink)?;
        }
        {
            let mut shared = self.shared.lock().expect("shared lock poisoned");
            shared.sink = Some(sink.clone());
        }
        self.sink = Some(sink.clone());
        self.refresh = Some(start_refresh_thread(Arc::clone(&self.state), sink));
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        self.refresh.take();
        if let Ok(mut shared) = self.shared.lock() {
            shared.sink = None;
        }
        self.sink = None;
        Ok(())
    }

    fn set_search_text(&mut self, value: String) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.search_text = value;
            state.rebuild_visible();
        };
        if let Some(sink) = sink {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            publish_snapshot(&mut state, &sink)?;
        }
        Ok(())
    }

    fn set_is_frozen(&mut self, value: bool) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.is_frozen = value;
        }
        if let Some(sink) = sink {
            sink.set_is_frozen(value)?;
            sink.set_freeze_action_label(if value { "Resume" } else { "Freeze" })?;
            sink.set_status(if value {
                "Refresh frozen."
            } else {
                "Live refresh running."
            })?;
        }
        if let Some(refresh) = &self.refresh {
            refresh.wake();
        }
        Ok(())
    }

    fn set_is_dark_theme(&mut self, value: bool) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.is_dark_theme = value;
        }
        if let Some(sink) = sink {
            sink.set_is_dark_theme(value)?;
            sink.set_theme_action_label(if value { "Light theme" } else { "Dark theme" })?;
        }
        Ok(())
    }

    fn set_selected_pid(&mut self, value: i64) -> Result<()> {
        let mut state = self.state.lock().expect("monitor state lock poisoned");
        let identity = if value > 0 {
            state
                .processes
                .iter()
                .find(|process| process.pid as i64 == value)
                .map(process_key)
        } else {
            None
        };
        state.selected_pid = if identity.is_some() { value } else { 0 };
        state.selected_key = identity.unwrap_or_default();
        state.restore_selection();
        Ok(())
    }

    fn set_selected_index(&mut self, value: i64) -> Result<()> {
        let (selected_key, selected_pid) = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.selected_index = value;
            if let Some(process) = state.selected_process().cloned() {
                state.selected_key = process_key(&process);
                state.selected_pid = process.pid as i64;
            } else {
                state.selected_key.clear();
                state.selected_pid = 0;
            }
            (state.selected_key.clone(), state.selected_pid)
        };
        if let Some(sink) = &self.sink {
            sink.set_selected_key(&selected_key)?;
            sink.set_selected_pid(selected_pid)?;
        }
        Ok(())
    }

    fn set_selected_key(&mut self, value: String) -> Result<()> {
        let mut state = self.state.lock().expect("monitor state lock poisoned");
        state.selected_key = value;
        let selected_pid = state
            .processes
            .iter()
            .find(|process| process_key(process) == state.selected_key)
            .map(|process| process.pid as i64)
            .unwrap_or(0);
        state.selected_pid = selected_pid;
        state.restore_selection();
        Ok(())
    }

    fn set_cpu_filter_enabled(&mut self, value: bool) -> Result<()> {
        self.update_filter(|filters| filters.cpu_enabled = value)
    }

    fn set_cpu_filter_operator(&mut self, value: String) -> Result<()> {
        self.update_filter(|filters| filters.cpu_operator = value)
    }

    fn set_cpu_filter_value(&mut self, value: i64) -> Result<()> {
        self.update_filter(|filters| filters.cpu_value = value as f32)
    }

    fn set_ram_filter_enabled(&mut self, value: bool) -> Result<()> {
        self.update_filter(|filters| filters.ram_enabled = value)
    }

    fn set_ram_filter_operator(&mut self, value: String) -> Result<()> {
        self.update_filter(|filters| filters.ram_operator = value)
    }

    fn set_ram_filter_value(&mut self, value: i64) -> Result<()> {
        self.update_filter(|filters| filters.ram_value = value as f64)
    }

    fn set_runtime_filter_enabled(&mut self, value: bool) -> Result<()> {
        self.update_filter(|filters| filters.runtime_enabled = value)
    }

    fn set_runtime_filter_operator(&mut self, value: String) -> Result<()> {
        self.update_filter(|filters| filters.runtime_operator = value)
    }

    fn set_runtime_filter_value(&mut self, value: i64) -> Result<()> {
        self.update_filter(|filters| filters.runtime_value = value as f64)
    }

    fn set_status_filter(&mut self, value: String) -> Result<()> {
        self.update_filter(|filters| filters.status = value)
    }

    fn refresh(&mut self) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            if let Err(error) = state.collect() {
                if let Some(sink) = &sink {
                    sink.set_error_message(Some(error))?;
                }
                return Ok(());
            }
        };
        if let Some(sink) = sink {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            publish_snapshot(&mut state, &sink)?;
        }
        Ok(())
    }

    fn kill(&mut self, value: String) -> Result<()> {
        let sink = self.sink.clone();
        let mut state = self.state.lock().expect("monitor state lock poisoned");
        let Some((pid, identity, name)) = state
            .visible_process_by_key(&value)
            .map(|process| (process.pid, process.identity, process.name.clone()))
        else {
            drop(state);
            if let Some(sink) = sink {
                sink.set_error_message(Some("Select a process to end."))?;
            }
            return Ok(());
        };
        let Some(pending) = ProcessMonitor::prepare_kill(&state.sys, pid, identity) else {
            drop(state);
            if let Some(sink) = sink {
                sink.set_error_message(Some(format!(
                    "Process {pid} ended or changed before confirmation."
                )))?;
            }
            return Ok(());
        };
        state.pending_kill = Some(pending);
        state.show_kill_confirm = true;
        state.kill_confirm_message = format!("Are you sure you want to end {name} (PID: {pid})?");
        drop(state);
        if let Some(sink) = sink {
            let state = self.state.lock().expect("monitor state lock poisoned");
            sink.set_show_kill_confirm(true)?;
            sink.set_kill_confirm_message(&state.kill_confirm_message)?;
        }
        Ok(())
    }

    fn pin(&mut self, value: String) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let command = state
                .visible_process_by_key(&value)
                .map(|process| process.command.clone());
            let Some(command) = command else {
                return Ok(());
            };
            if !state.pinned.remove(&command) {
                state.pinned.insert(command);
            }
            state.rebuild_visible();
        };
        if let Some(sink) = sink {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            publish_snapshot(&mut state, &sink)?;
        }
        Ok(())
    }

    fn show_details(&mut self, value: String) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let process = state.visible_process_by_key(&value).cloned();
            let Some(process) = process else {
                return Ok(());
            };
            state.detail_pid = process.pid;
            state.show_details = true;
            if let Some(sink) = &sink {
                publish_details(&state, sink, &process)?;
            }
        }
        Ok(())
    }

    fn toggle_theme(&mut self) -> Result<()> {
        let next = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.is_dark_theme = !state.is_dark_theme;
            state.is_dark_theme
        };
        self.set_is_dark_theme(next)
    }

    fn toggle_freeze(&mut self) -> Result<()> {
        let next = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.is_frozen = !state.is_frozen;
            state.is_frozen
        };
        self.set_is_frozen(next)
    }

    fn sort_processes(&mut self, value: String) -> Result<()> {
        let column = value.split(':').next().and_then(SortColumn::parse).ok_or(
            Error::InvalidViewModelMember {
                kind: "sort",
                id: 7,
            },
        )?;
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            if state.sort_column == column {
                state.sort_descending = !state.sort_descending;
            } else {
                state.sort_column = column;
                state.sort_descending = true;
            }
            state.rebuild_visible();
        };
        if let Some(sink) = sink {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            publish_snapshot(&mut state, &sink)?;
        }
        Ok(())
    }

    fn confirm_kill(&mut self) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.show_kill_confirm = false;
            let Some(pending) = state.pending_kill.take() else {
                return Ok(());
            };
            let pid = pending.pid();
            state.sys.refresh_all();
            let killed = ProcessMonitor::kill_process(&state.sys, pending);
            state.kill_confirm_message.clear();
            if !killed {
                if let Some(sink) = &sink {
                    sink.set_show_kill_confirm(false)?;
                    sink.set_error_message(Some(format!(
                        "Process {pid} ended or changed before confirmation."
                    )))?;
                }
                return Ok(());
            }
            let _ = state.collect();
        };
        if let Some(sink) = sink {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            sink.set_show_kill_confirm(false)?;
            publish_snapshot(&mut state, &sink)?;
        }
        Ok(())
    }

    fn cancel_kill(&mut self) -> Result<()> {
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.show_kill_confirm = false;
            state.pending_kill = None;
            state.kill_confirm_message.clear();
        }
        if let Some(sink) = &self.sink {
            sink.set_show_kill_confirm(false)?;
        }
        Ok(())
    }

    fn close_details(&mut self) -> Result<()> {
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.show_details = false;
        }
        if let Some(sink) = &self.sink {
            sink.set_show_details(false)?;
        }
        Ok(())
    }

    fn show_parent_details(&mut self) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let parent = state
                .process_by_pid(state.detail_pid)
                .map(|process| process.ppid)
                .and_then(|ppid| state.process_by_pid(ppid).cloned());
            let Some(process) = parent else {
                return Ok(());
            };
            state.detail_pid = process.pid;
            if let Some(sink) = &sink {
                publish_details(&state, sink, &process)?;
            }
        }
        Ok(())
    }

    fn copy_selected_row(&mut self) -> Result<()> {
        let text = {
            let state = self.state.lock().expect("monitor state lock poisoned");
            state
                .selected_process()
                .map(|process| {
                    format!(
                        "{}\t{}\t{}\t{}\t{}",
                        process.name,
                        process.pid,
                        process.status,
                        format::cpu(process.cpu_usage),
                        format::bytes(process.memory_usage)
                    )
                })
                .unwrap_or_default()
        };
        let shared = self.shared.lock().expect("shared lock poisoned");
        let (Some(scope), Some(window), Some(sink)) = (
            shared.scope.clone(),
            shared.window.clone(),
            shared.sink.clone(),
        ) else {
            return Ok(());
        };
        drop(shared);
        let operation = scope.clipboard_write(&window, &ClipboardData::text(text))?;
        scope.spawn(async move {
            let status = match operation.await {
                Ok(()) => "Copied selected process row.".to_owned(),
                Err(error) => format!("Clipboard write failed: {error}"),
            };
            let _ = sink.set_status(status);
        })
    }

    fn exit_application(&mut self) -> Result<()> {
        let scope = self
            .shared
            .lock()
            .expect("shared lock poisoned")
            .scope
            .clone();
        match scope {
            Some(scope) => scope.shutdown(),
            None => Ok(()),
        }
    }

    fn clear_search(&mut self) -> Result<()> {
        self.set_search_text(String::new())?;
        if let Some(sink) = &self.sink {
            sink.set_search_text("")?;
        }
        Ok(())
    }

    fn toggle_cpu_operator(&mut self) -> Result<()> {
        let next = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let next = filter::toggle_operator(&state.filters.cpu_operator).to_owned();
            state.filters.cpu_operator = next.clone();
            next
        };
        if let Some(sink) = &self.sink {
            sink.set_cpu_filter_operator(&next)?;
        }
        self.update_filter(|_| {})
    }

    fn toggle_ram_operator(&mut self) -> Result<()> {
        let next = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let next = filter::toggle_operator(&state.filters.ram_operator).to_owned();
            state.filters.ram_operator = next.clone();
            next
        };
        if let Some(sink) = &self.sink {
            sink.set_ram_filter_operator(&next)?;
        }
        self.update_filter(|_| {})
    }

    fn toggle_runtime_operator(&mut self) -> Result<()> {
        let next = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let next = filter::toggle_operator(&state.filters.runtime_operator).to_owned();
            state.filters.runtime_operator = next.clone();
            next
        };
        if let Some(sink) = &self.sink {
            sink.set_runtime_filter_operator(&next)?;
        }
        self.update_filter(|_| {})
    }

    fn set_refresh_rate_ms(&mut self, value: i64) -> Result<()> {
        let ms = if REFRESH_RATES.contains(&(value as u64)) {
            value as u64
        } else {
            3000
        };
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.refresh_ms = ms;
        }
        if let Some(sink) = &self.sink {
            sink.set_refresh_rate_ms(ms as i64)?;
            sink.set_refresh_rate_label(refresh_label(ms))?;
        }
        if let Some(refresh) = &self.refresh {
            refresh.wake();
        }
        Ok(())
    }

    fn set_show_filters(&mut self, value: bool) -> Result<()> {
        self.set_overlay(Overlay::Filters, value)
    }

    fn set_show_columns(&mut self, value: bool) -> Result<()> {
        self.set_overlay(Overlay::Columns, value)
    }

    fn set_show_search_help(&mut self, value: bool) -> Result<()> {
        self.set_overlay(Overlay::SearchHelp, value)
    }

    fn set_show_pid(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.pid = value)
    }
    fn set_show_status(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.status = value)
    }
    fn set_show_user(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.user = value)
    }
    fn set_show_cpu(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.cpu = value)
    }
    fn set_show_ram(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.ram = value)
    }
    fn set_show_virt(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.virt = value)
    }
    fn set_show_disk(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.disk = value)
    }
    fn set_show_run_time(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.run_time = value)
    }
    fn set_show_command(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.command = value)
    }
    fn set_show_ppid(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.ppid = value)
    }
    fn set_show_root(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.root = value)
    }
    fn set_show_environ(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.environ = value)
    }
    fn set_show_session(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.session = value)
    }
    fn set_show_start_time(&mut self, value: bool) -> Result<()> {
        self.set_column(|columns| columns.start_time = value)
    }

    fn toggle_filters(&mut self) -> Result<()> {
        let next = {
            let state = self.state.lock().expect("monitor state lock poisoned");
            !state.show_filters
        };
        self.set_overlay(Overlay::Filters, next)
    }

    fn toggle_columns(&mut self) -> Result<()> {
        let next = {
            let state = self.state.lock().expect("monitor state lock poisoned");
            !state.show_columns
        };
        self.set_overlay(Overlay::Columns, next)
    }

    fn toggle_search_help(&mut self) -> Result<()> {
        let next = {
            let state = self.state.lock().expect("monitor state lock poisoned");
            !state.show_search_help
        };
        self.set_overlay(Overlay::SearchHelp, next)
    }

    fn cycle_refresh_rate(&mut self) -> Result<()> {
        let next = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            let index = REFRESH_RATES
                .iter()
                .position(|&rate| rate == state.refresh_ms)
                .unwrap_or(1);
            let next = REFRESH_RATES[(index + 1) % REFRESH_RATES.len()];
            state.refresh_ms = next;
            next
        };
        if let Some(sink) = &self.sink {
            sink.set_refresh_rate_ms(next as i64)?;
            sink.set_refresh_rate_label(refresh_label(next))?;
        }
        if let Some(refresh) = &self.refresh {
            refresh.wake();
        }
        Ok(())
    }

    fn close_overlays(&mut self) -> Result<()> {
        self.set_overlay(Overlay::None, false)
    }

}

#[derive(Clone, Copy)]
enum Overlay {
    None,
    Filters,
    Columns,
    SearchHelp,
}

impl Model {
    fn set_overlay(&mut self, which: Overlay, enabled: bool) -> Result<()> {
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            state.show_filters = enabled && matches!(which, Overlay::Filters);
            state.show_columns = enabled && matches!(which, Overlay::Columns);
            state.show_search_help = enabled && matches!(which, Overlay::SearchHelp);
        }
        if let Some(sink) = &self.sink {
            let state = self.state.lock().expect("monitor state lock poisoned");
            sink.set_show_filters(state.show_filters)?;
            sink.set_show_columns(state.show_columns)?;
            sink.set_show_search_help(state.show_search_help)?;
        }
        Ok(())
    }

    fn set_column(&mut self, update: impl FnOnce(&mut ColumnVisibility)) -> Result<()> {
        let columns = {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            update(&mut state.columns);
            state.columns
        };
        if let Some(sink) = &self.sink {
            publish_columns(sink, columns)?;
        }
        Ok(())
    }

    fn update_filter(&mut self, update: impl FnOnce(&mut Filters)) -> Result<()> {
        let sink = self.sink.clone();
        {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            update(&mut state.filters);
            state.rebuild_visible();
        };
        if let Some(sink) = sink {
            let mut state = self.state.lock().expect("monitor state lock poisoned");
            publish_snapshot(&mut state, &sink)?;
        }
        Ok(())
    }
}

fn start_refresh_thread(
    state: Arc<Mutex<MonitorState>>,
    sink: MainViewModelSink,
) -> refresh::RefreshWorker {
    let interval_state = Arc::clone(&state);
    refresh::RefreshWorker::start(
        move || {
            Duration::from_millis(
                interval_state
                    .lock()
                    .expect("monitor state lock poisoned")
                    .refresh_ms,
            )
        },
        move || {
            let outcome = {
                let mut state = state.lock().expect("monitor state lock poisoned");
                if state.is_frozen {
                    None
                } else {
                    Some(state.collect())
                }
            };
            match outcome {
                None => {}
                Some(Err(error)) => {
                    let _ = sink.set_error_message(Some(error));
                }
                Some(Ok(())) => {
                    let mut state = state.lock().expect("monitor state lock poisoned");
                    let _ = publish_snapshot(&mut state, &sink);
                }
            }
        },
    )
}

fn publish_snapshot(
    state: &mut MonitorState,
    sink: &MainViewModelSink,
) -> Result<()> {
    let cpu_avg = if state.stats.cpu_usage.is_empty() {
        0.0
    } else {
        f64::from(state.stats.cpu_usage.iter().sum::<f32>()) / state.stats.cpu_usage.len() as f64
    };
    let memory_percent = format::percent(state.stats.memory_used, state.stats.memory_total);
    let storage_percent =
        format::percent(state.stats.disk_used_bytes, state.stats.disk_total_bytes);
    let mut batch = sink.batch(state.next_batch_generation());
    batch.set_search_text(&state.search_text);
    batch.set_is_frozen(state.is_frozen);
    batch.set_is_dark_theme(state.is_dark_theme);
    batch.set_cpu_percent(cpu_avg);
    batch.set_memory_percent(memory_percent);
    batch.set_memory_used(state.stats.memory_used as i64);
    batch.set_memory_total(state.stats.memory_total as i64);
    batch.set_process_count(state.visible.len() as i64);
    batch.set_status(format!(
        "{} processes · refresh {}",
        state.visible.len(),
        refresh_label(state.refresh_ms)
    ));
    batch.set_selected_pid(state.selected_pid);
    batch.set_selected_index(state.selected_index);
    batch.set_selected_key(&state.selected_key);
    batch.set_sort_direction(sort_label(state.sort_column, state.sort_descending));
    batch.set_error_message(None::<&str>);
    batch.set_cpu_summary(format!(
        "CPU {} avg · {} cores",
        format::cpu(cpu_avg as f32),
        state.stats.cpu_usage.len()
    ));
    batch.set_memory_summary(format!(
        "Memory {} / {} ({} free, {} cached)",
        format::bytes(state.stats.memory_used),
        format::bytes(state.stats.memory_total),
        format::bytes(state.stats.memory_free),
        format::bytes(state.stats.memory_cached)
    ));
    batch.set_network_summary(format!(
        "Network ↓ {}/s  ↑ {}/s",
        format::bytes(state.stats.network_rx_bytes),
        format::bytes(state.stats.network_tx_bytes)
    ));
    batch.set_storage_summary(format!(
        "Storage {} / {} ({} free)",
        format::bytes(state.stats.disk_used_bytes),
        format::bytes(state.stats.disk_total_bytes),
        format::bytes(state.stats.disk_free_bytes)
    ));
    batch.set_storage_percent(storage_percent);
    batch.set_system_summary(format!(
        "Up {} · load {:.2} {:.2} {:.2}",
        format::uptime(state.stats.uptime),
        state.stats.load_avg[0],
        state.stats.load_avg[1],
        state.stats.load_avg[2]
    ));
    batch.set_freeze_action_label(if state.is_frozen { "Resume" } else { "Freeze" });
    batch.set_theme_action_label(if state.is_dark_theme {
        "Light theme"
    } else {
        "Dark theme"
    });
    batch.set_show_kill_confirm(state.show_kill_confirm);
    batch.set_kill_confirm_message(&state.kill_confirm_message);
    batch.set_show_details(state.show_details);
    batch.set_process_count_label(format!(
        "{} {}",
        state.visible.len(),
        if state.visible.len() == 1 {
            "process"
        } else {
            "processes"
        }
    ));
    batch.set_memory_free(state.stats.memory_free as i64);
    batch.set_network_rx(state.stats.network_rx_bytes as i64);
    batch.set_network_tx(state.stats.network_tx_bytes as i64);
    batch.set_storage_used(state.stats.disk_used_bytes as i64);
    batch.set_storage_total(state.stats.disk_total_bytes as i64);
    batch.set_storage_free(state.stats.disk_free_bytes as i64);
    batch.set_refresh_rate_ms(state.refresh_ms as i64);
    batch.set_refresh_rate_label(refresh_label(state.refresh_ms));
    batch.set_memory_used_label(format::bytes(state.stats.memory_used));
    batch.set_memory_total_label(format::bytes(state.stats.memory_total));
    batch.set_memory_free_label(format::bytes(state.stats.memory_free));
    batch.set_network_rx_label(format!("{}/s", format::bytes(state.stats.network_rx_bytes)));
    batch.set_network_tx_label(format!("{}/s", format::bytes(state.stats.network_tx_bytes)));
    batch.set_storage_used_label(format::bytes(state.stats.disk_used_bytes));
    batch.set_storage_total_label(format::bytes(state.stats.disk_total_bytes));
    batch.set_storage_free_label(format::bytes(state.stats.disk_free_bytes));
    batch.set_uptime_label(format::uptime(state.stats.uptime));
    batch.set_load_one_label(format!("{:.2}", state.stats.load_avg[0]));
    batch.set_load_five_label(format!("{:.2}", state.stats.load_avg[1]));
    batch.set_load_fifteen_label(format!("{:.2}", state.stats.load_avg[2]));
    batch.set_show_filters(state.show_filters);
    batch.set_show_columns(state.show_columns);
    batch.set_show_search_help(state.show_search_help);
    batch.set_show_pid(state.columns.pid);
    batch.set_show_status(state.columns.status);
    batch.set_show_user(state.columns.user);
    batch.set_show_cpu(state.columns.cpu);
    batch.set_show_ram(state.columns.ram);
    batch.set_show_virt(state.columns.virt);
    batch.set_show_disk(state.columns.disk);
    batch.set_show_run_time(state.columns.run_time);
    batch.set_show_command(state.columns.command);
    batch.set_show_ppid(state.columns.ppid);
    batch.set_show_root(state.columns.root);
    batch.set_show_environ(state.columns.environ);
    batch.set_show_session(state.columns.session);
    batch.set_show_start_time(state.columns.start_time);
    batch.set_cpu_percent_label(format::cpu(cpu_avg as f32));
    batch.set_memory_percent_label(format!("{memory_percent:.1}%"));
    batch.set_storage_percent_label(format!("{storage_percent:.1}%"));
    batch.replace_cpu_cores_snapshot(core_rows(&state.stats.cpu_usage));
    let desired: Vec<_> = state
        .visible
        .iter()
        .map(|&index| process_key(&state.processes[index]))
        .collect();
    for (&index, key) in state.visible.iter().zip(&desired) {
        let process = &state.processes[index];
        let values = RowValues {
            process: process.clone(),
            pinned: state.pinned.contains(&process.command),
            high_usage: process.cpu_usage > 50.0
                || (state.stats.memory_total > 0
                    && process.memory_usage > state.stats.memory_total / 10),
        };
        if let Some(row) = state.published_rows.get(key) {
            row.update(values)?;
        } else {
            state.published_rows.insert(key.clone(), RowModel::new(values));
        }
    }
    for change in reconcile::reconcile(&mut state.published_keys, &desired) {
        match change {
            reconcile::Change::Remove(index) => batch.remove_processes(index as i32),
            reconcile::Change::Insert(index) => {
                batch.insert_processes(index as i32, state.published_rows[&desired[index]].clone());
            }
            reconcile::Change::Move { from, to } => batch.move_processes(from as i32, to as i32),
        }
    }
    let wanted: HashSet<_> = desired.iter().collect();
    state.published_rows.retain(|key, _| wanted.contains(key));
    sink.submit_batch(batch)?;
    Ok(())
}

fn core_rows(usage: &[f32]) -> Vec<CoreRow> {
    usage
        .iter()
        .enumerate()
        .map(|(index, value)| CoreRow {
            label: format!("Core {index}"),
            usage: f64::from(*value),
        })
        .collect()
}

fn publish_columns(sink: &MainViewModelSink, columns: ColumnVisibility) -> Result<()> {
    sink.set_show_pid(columns.pid)?;
    sink.set_show_status(columns.status)?;
    sink.set_show_user(columns.user)?;
    sink.set_show_cpu(columns.cpu)?;
    sink.set_show_ram(columns.ram)?;
    sink.set_show_virt(columns.virt)?;
    sink.set_show_disk(columns.disk)?;
    sink.set_show_run_time(columns.run_time)?;
    sink.set_show_command(columns.command)?;
    sink.set_show_ppid(columns.ppid)?;
    sink.set_show_root(columns.root)?;
    sink.set_show_environ(columns.environ)?;
    sink.set_show_session(columns.session)?;
    sink.set_show_start_time(columns.start_time)
}

fn publish_details(
    state: &MonitorState,
    sink: &MainViewModelSink,
    process: &ProcessInfo,
) -> Result<()> {
    let children: Vec<String> = state
        .processes
        .iter()
        .filter(|child| child.ppid == process.pid)
        .map(|child| {
            format!(
                "{:<28} {:>7} {:>7} {:>10}",
                truncate_name(&child.name, 28),
                child.pid,
                format::cpu(child.cpu_usage),
                format::bytes(child.memory_usage)
            )
        })
        .collect();
    sink.set_show_details(true)?;
    let title_name: String = process.name.chars().take(10).collect();
    sink.set_detail_title(format!("{title_name} - Process Details"))?;
    sink.set_show_parent_enabled(
        process.ppid != 0 && state.process_by_pid(process.ppid).is_some(),
    )?;
    sink.set_detail_name(&process.name)?;
    sink.set_detail_pid_label(process.pid.to_string())?;
    sink.set_detail_ppid_label(process.ppid.to_string())?;
    sink.set_detail_user(&process.user)?;
    sink.set_detail_status(&process.status)?;
    sink.set_detail_session_label(
        process
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".to_owned()),
    )?;
    sink.set_detail_cpu_label(format::cpu(process.cpu_usage))?;
    sink.set_detail_memory_label(format::bytes(process.memory_usage))?;
    sink.set_detail_virt_label(format::bytes(process.virtual_memory))?;
    sink.set_detail_disk_label(format!(
        "Read: {}    Written: {}",
        format::bytes(process.disk_usage.0),
        format::bytes(process.disk_usage.1)
    ))?;
    sink.set_detail_command(&process.command)?;
    sink.set_detail_root(&process.root)?;
    sink.set_detail_children(if children.is_empty() {
        "No child processes".to_owned()
    } else {
        let mut table = String::from("Name                             PID     CPU     Memory\n");
        table.push_str(&children.join("\n"));
        table
    })?;
    sink.set_detail_body(if process.environ.is_empty() {
        "No environment variables".to_owned()
    } else {
        process.environ.join("\n")
    })
}

fn truncate_name(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        name.to_owned()
    } else {
        name.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

struct CoreRow {
    label: String,
    usage: f64,
}

impl CpuCoreViewModel for CoreRow {
    fn attach(&mut self, sink: CpuCoreViewModelSink) -> Result<()> {
        sink.set_label(&self.label)?;
        sink.set_usage(self.usage)
    }

    fn detach(&mut self) -> Result<()> {
        Ok(())
    }
}

struct RowValues {
    process: ProcessInfo,
    pinned: bool,
    high_usage: bool,
}

struct RowState {
    values: RowValues,
    sink: Option<ProcessRowViewModelSink>,
    generation: i64,
}

#[derive(Clone)]
struct RowModel(Arc<Mutex<RowState>>);

impl RowModel {
    fn new(values: RowValues) -> Self {
        Self(Arc::new(Mutex::new(RowState {
            values,
            sink: None,
            generation: 1,
        })))
    }

    fn update(&self, values: RowValues) -> Result<()> {
        let mut state = self.0.lock().expect("process row lock poisoned");
        state.values = values;
        state.generation = state.generation.saturating_add(1);
        if let Some(sink) = &state.sink {
            state.values.publish(sink, state.generation)?;
        }
        Ok(())
    }
}

impl ProcessRowViewModel for RowModel {
    fn attach(&mut self, sink: ProcessRowViewModelSink) -> Result<()> {
        let mut state = self.0.lock().expect("process row lock poisoned");
        state.values.publish(&sink, state.generation)?;
        state.sink = Some(sink);
        Ok(())
    }

    fn detach(&mut self) -> Result<()> {
        self.0.lock().expect("process row lock poisoned").sink = None;
        Ok(())
    }
}

impl RowValues {
    fn publish(&self, sink: &ProcessRowViewModelSink, generation: i64) -> Result<()> {
        let process = &self.process;
        let mut batch = sink.batch(generation);
        batch.set_name(&process.name);
        batch.set_pid(process.pid as i64);
        batch.set_ppid(process.ppid as i64);
        batch.set_status(&process.status);
        batch.set_user(&process.user);
        batch.set_cpu_usage(f64::from(process.cpu_usage));
        batch.set_memory_usage(process.memory_usage as i64);
        batch.set_virtual_memory(process.virtual_memory as i64);
        batch.set_disk_read(process.disk_usage.0 as i64);
        batch.set_disk_write(process.disk_usage.1 as i64);
        batch.set_command(&process.command);
        batch.set_root(&process.root);
        batch.set_session_id(process.session_id.unwrap_or(0) as i64);
        batch.set_start_time(process.start_time as i64);
        batch.set_run_time(process.run_time as i64);
        batch.set_is_pinned(self.pinned);
        batch.set_key(process_key(process));
        batch.set_environ(process.environ.join("; "));
        batch.set_disk_io(format::disk_io(process.disk_usage.0, process.disk_usage.1));
        batch.set_run_time_label(format::runtime(process.run_time));
        batch.set_memory_label(format::bytes(process.memory_usage));
        batch.set_virtual_memory_label(format::bytes(process.virtual_memory));
        batch.set_cpu_label(format::cpu(process.cpu_usage));
        batch.set_pin_label(if self.pinned { "Unpin" } else { "Pin" });
        batch.set_is_high_usage(self.high_usage);
        batch.set_start_time_label(format::start_time(process.start_time));
        batch.set_session_label(
            process
                .session_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "-".to_owned()),
        );
        batch.set_icon_png(crate::icon::png_for_process(
            process.pid,
            &process.exe_path,
            &process.command,
        ));
        sink.submit_batch(batch).map(|_| ())
    }
}

fn main() -> avalonia::Result<()> {
    App::load_from_env()?.run(|scope| {
        let state = Arc::new(Mutex::new(MonitorState::new()));
        let shared = Arc::new(Mutex::new(Shared::default()));
        mount_main_window(scope, Model::new(Arc::clone(&state), Arc::clone(&shared)))?;
        let mut shared = shared.lock().expect("shared lock poisoned");
        shared.scope = Some(scope.clone());
        shared.window = scope.main_window();
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(pid: u32, name: &str, cpu_usage: f32) -> ProcessInfo {
        ProcessInfo {
            pid,
            identity: pid as u64,
            ppid: 0,
            name: name.into(),
            cpu_usage,
            memory_usage: 1024,
            status: "Running".into(),
            user: "fixture".into(),
            command: name.into(),
            threads: None,
            environ: vec![],
            root: String::new(),
            virtual_memory: 0,
            start_time: 100,
            run_time: 1,
            disk_usage: (0, 0),
            session_id: None,
            exe_path: String::new(),
        }
    }

    #[test]
    fn selection_survives_sort_and_filter_but_hidden_selection_is_not_actionable() {
        let mut state = MonitorState::with_monitors(System::new(), Disks::new(), Networks::new());
        state.processes = vec![fixture(1001, "alpha", 20.0), fixture(1002, "beta", 80.0)];
        state.rebuild_visible();
        assert_eq!(state.visible, vec![1, 0]);
        assert!(state.selected_process().is_none());
        state.selected_key = process_key(&state.processes[0]);
        state.selected_pid = 1001;
        state.sort_descending = false;
        state.rebuild_visible();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.selected_process().unwrap().pid, 1001);
        state.search_text = "beta".into();
        state.rebuild_visible();
        assert_eq!(state.visible, vec![1]);
        assert_eq!(state.selected_index, -1);
        assert!(state.selected_process().is_none());
        state.search_text.clear();
        state.rebuild_visible();
        assert_eq!(state.selected_process().unwrap().pid, 1001);
        state.processes.remove(0);
        state.rebuild_visible();
        assert!(state.selected_process().is_none());
    }

    #[test]
    fn no_selection_cannot_target_first_row_for_process_action() {
        let mut state = MonitorState::with_monitors(System::new(), Disks::new(), Networks::new());
        state.processes = vec![fixture(1001, "alpha", 20.0)];
        state.rebuild_visible();
        let state = Arc::new(Mutex::new(state));
        let mut model = Model::new(Arc::clone(&state), Arc::new(Mutex::new(Shared::default())));
        model.set_selected_index(-1).unwrap();
        model.kill(String::new()).unwrap();
        {
            let state = state.lock().unwrap();
            assert_eq!(state.selected_pid, 0);
            assert!(!state.show_kill_confirm);
            assert!(state.pending_kill.is_none());
        }
        model.set_selected_index(0).unwrap();
        model.set_search_text("absent".into()).unwrap();
        let selected_key = state.lock().unwrap().selected_key.clone();
        model.kill(selected_key).unwrap();
        let state = state.lock().unwrap();
        assert_eq!(state.selected_index, -1);
        assert!(!state.show_kill_confirm);
        assert!(state.pending_kill.is_none());
    }
}
