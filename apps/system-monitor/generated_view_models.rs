//! Generated from view-model.ir.json. Do not edit.

#![allow(dead_code)]

#[derive(Clone, Debug)]
pub struct MainViewModelSink(crate::view_model::ViewModelSink);

impl MainViewModelSink {
    pub fn set_search_text(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_is_frozen(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(2, value) }
    pub fn set_is_dark_theme(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(3, value) }
    pub fn set_cpu_percent(&self, value: f64) -> crate::Result<()> { self.0.set_double(4, value) }
    pub fn set_memory_percent(&self, value: f64) -> crate::Result<()> { self.0.set_double(5, value) }
    pub fn set_memory_used(&self, value: i64) -> crate::Result<()> { self.0.set_integer(6, value) }
    pub fn set_memory_total(&self, value: i64) -> crate::Result<()> { self.0.set_integer(7, value) }
    pub fn set_process_count(&self, value: i64) -> crate::Result<()> { self.0.set_integer(8, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(9, value) }
    pub fn set_selected_pid(&self, value: i64) -> crate::Result<()> { self.0.set_integer(10, value) }
    pub fn set_selected_index(&self, value: i64) -> crate::Result<()> { self.0.set_integer(11, value) }
    pub fn set_selected_key(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(12, value) }
    pub fn set_sort_direction(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(13, value) }
    pub fn set_error_message(&self, value: Option<impl AsRef<str>>) -> crate::Result<()> { match value { Some(value) => self.0.set_string(14, value), None => self.0.set_null(14) } }
    pub fn set_cpu_summary(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(15, value) }
    pub fn set_memory_summary(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(16, value) }
    pub fn set_network_summary(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(17, value) }
    pub fn set_storage_summary(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(18, value) }
    pub fn set_storage_percent(&self, value: f64) -> crate::Result<()> { self.0.set_double(19, value) }
    pub fn set_system_summary(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(20, value) }
    pub fn set_freeze_action_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(21, value) }
    pub fn set_theme_action_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(22, value) }
    pub fn set_show_kill_confirm(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(23, value) }
    pub fn set_kill_confirm_message(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(24, value) }
    pub fn set_is_killing(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(25, value) }
    pub fn set_show_details(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(26, value) }
    pub fn set_detail_title(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(27, value) }
    pub fn set_detail_body(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(28, value) }
    pub fn set_show_parent_enabled(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(29, value) }
    pub fn set_cpu_filter_enabled(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(30, value) }
    pub fn set_cpu_filter_operator(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(31, value) }
    pub fn set_cpu_filter_value(&self, value: i64) -> crate::Result<()> { self.0.set_integer(32, value) }
    pub fn set_ram_filter_enabled(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(33, value) }
    pub fn set_ram_filter_operator(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(34, value) }
    pub fn set_ram_filter_value(&self, value: i64) -> crate::Result<()> { self.0.set_integer(35, value) }
    pub fn set_runtime_filter_enabled(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(36, value) }
    pub fn set_runtime_filter_operator(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(37, value) }
    pub fn set_runtime_filter_value(&self, value: i64) -> crate::Result<()> { self.0.set_integer(38, value) }
    pub fn set_status_filter(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(39, value) }
    pub fn set_process_count_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(40, value) }
    pub fn set_memory_free(&self, value: i64) -> crate::Result<()> { self.0.set_integer(41, value) }
    pub fn set_network_rx(&self, value: i64) -> crate::Result<()> { self.0.set_integer(42, value) }
    pub fn set_network_tx(&self, value: i64) -> crate::Result<()> { self.0.set_integer(43, value) }
    pub fn set_storage_used(&self, value: i64) -> crate::Result<()> { self.0.set_integer(44, value) }
    pub fn set_storage_total(&self, value: i64) -> crate::Result<()> { self.0.set_integer(45, value) }
    pub fn set_storage_free(&self, value: i64) -> crate::Result<()> { self.0.set_integer(46, value) }
    pub fn set_refresh_rate_ms(&self, value: i64) -> crate::Result<()> { self.0.set_integer(47, value) }
    pub fn set_refresh_rate_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(48, value) }
    pub fn set_memory_used_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(49, value) }
    pub fn set_memory_total_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(50, value) }
    pub fn set_memory_free_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(51, value) }
    pub fn set_network_rx_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(52, value) }
    pub fn set_network_tx_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(53, value) }
    pub fn set_storage_used_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(54, value) }
    pub fn set_storage_total_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(55, value) }
    pub fn set_storage_free_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(56, value) }
    pub fn set_uptime_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(57, value) }
    pub fn set_load_one_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(58, value) }
    pub fn set_load_five_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(59, value) }
    pub fn set_load_fifteen_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(60, value) }
    pub fn set_show_filters(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(61, value) }
    pub fn set_show_columns(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(62, value) }
    pub fn set_show_search_help(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(63, value) }
    pub fn set_show_pid(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(64, value) }
    pub fn set_show_status(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(65, value) }
    pub fn set_show_user(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(66, value) }
    pub fn set_show_cpu(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(67, value) }
    pub fn set_show_ram(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(68, value) }
    pub fn set_show_virt(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(69, value) }
    pub fn set_show_disk(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(70, value) }
    pub fn set_show_run_time(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(71, value) }
    pub fn set_show_command(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(72, value) }
    pub fn set_show_ppid(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(73, value) }
    pub fn set_show_root(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(74, value) }
    pub fn set_show_environ(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(75, value) }
    pub fn set_show_session(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(76, value) }
    pub fn set_show_start_time(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(77, value) }
    pub fn set_detail_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(78, value) }
    pub fn set_detail_pid_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(79, value) }
    pub fn set_detail_ppid_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(80, value) }
    pub fn set_detail_user(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(81, value) }
    pub fn set_detail_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(82, value) }
    pub fn set_detail_session_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(83, value) }
    pub fn set_detail_cpu_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(84, value) }
    pub fn set_detail_memory_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(85, value) }
    pub fn set_detail_virt_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(86, value) }
    pub fn set_detail_disk_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(87, value) }
    pub fn set_detail_command(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(88, value) }
    pub fn set_detail_root(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(89, value) }
    pub fn set_detail_children(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(90, value) }
    pub fn set_cpu_percent_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(91, value) }
    pub fn set_memory_percent_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(92, value) }
    pub fn set_storage_percent_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(93, value) }
    pub fn add_processes(&self, value: impl ProcessRowViewModel) -> crate::Result<()> { self.0.add_model(1, ProcessRowViewModelDispatch { model: value }) }
    pub fn insert_processes(&self, index: i32, value: impl ProcessRowViewModel) -> crate::Result<()> { self.0.insert_model(1, index, ProcessRowViewModelDispatch { model: value }) }
    pub fn replace_processes(&self, index: i32, value: impl ProcessRowViewModel) -> crate::Result<()> { self.0.replace_model(1, index, ProcessRowViewModelDispatch { model: value }) }
    pub fn add_cpu_cores(&self, value: impl CpuCoreViewModel) -> crate::Result<()> { self.0.add_model(2, CpuCoreViewModelDispatch { model: value }) }
    pub fn insert_cpu_cores(&self, index: i32, value: impl CpuCoreViewModel) -> crate::Result<()> { self.0.insert_model(2, index, CpuCoreViewModelDispatch { model: value }) }
    pub fn replace_cpu_cores(&self, index: i32, value: impl CpuCoreViewModel) -> crate::Result<()> { self.0.replace_model(2, index, CpuCoreViewModelDispatch { model: value }) }
    pub fn remove_processes(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(1, index) }
    pub fn move_processes(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(1, from_index, to_index) }
    pub fn clear_processes(&self) -> crate::Result<()> { self.0.clear_model_collection(1) }
    pub fn remove_cpu_cores(&self, index: i32) -> crate::Result<()> { self.0.remove_model_at(2, index) }
    pub fn move_cpu_cores(&self, from_index: i32, to_index: i32) -> crate::Result<()> { self.0.move_model_item(2, from_index, to_index) }
    pub fn clear_cpu_cores(&self) -> crate::Result<()> { self.0.clear_model_collection(2) }
    pub fn set_refresh_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(1, enabled) }
    pub fn set_kill_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(2, enabled) }
    pub fn set_pin_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(3, enabled) }
    pub fn set_show_details_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(4, enabled) }
    pub fn set_toggle_theme_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(5, enabled) }
    pub fn set_toggle_freeze_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(6, enabled) }
    pub fn set_sort_processes_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(7, enabled) }
    pub fn set_confirm_kill_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(8, enabled) }
    pub fn set_cancel_kill_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(9, enabled) }
    pub fn set_close_details_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(10, enabled) }
    pub fn set_show_parent_details_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(11, enabled) }
    pub fn set_copy_selected_row_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(12, enabled) }
    pub fn set_exit_application_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(13, enabled) }
    pub fn set_clear_search_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(14, enabled) }
    pub fn set_toggle_cpu_operator_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(15, enabled) }
    pub fn set_toggle_ram_operator_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(16, enabled) }
    pub fn set_toggle_runtime_operator_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(17, enabled) }
    pub fn set_toggle_filters_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(18, enabled) }
    pub fn set_toggle_columns_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(19, enabled) }
    pub fn set_toggle_search_help_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(20, enabled) }
    pub fn set_cycle_refresh_rate_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(21, enabled) }
    pub fn set_close_overlays_enabled(&self, enabled: bool) -> crate::Result<()> { self.0.set_command_enabled(22, enabled) }
    pub fn set_search_text_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_is_frozen_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_is_dark_theme_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_cpu_percent_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_memory_percent_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_memory_used_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_memory_total_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_process_count_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_selected_pid_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_selected_index_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_selected_key_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_sort_direction_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_error_message_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_cpu_summary_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
    pub fn set_memory_summary_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(16, message) }
    pub fn set_network_summary_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(17, message) }
    pub fn set_storage_summary_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(18, message) }
    pub fn set_storage_percent_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(19, message) }
    pub fn set_system_summary_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(20, message) }
    pub fn set_freeze_action_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(21, message) }
    pub fn set_theme_action_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(22, message) }
    pub fn set_show_kill_confirm_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(23, message) }
    pub fn set_kill_confirm_message_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(24, message) }
    pub fn set_is_killing_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(25, message) }
    pub fn set_show_details_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(26, message) }
    pub fn set_detail_title_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(27, message) }
    pub fn set_detail_body_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(28, message) }
    pub fn set_show_parent_enabled_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(29, message) }
    pub fn set_cpu_filter_enabled_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(30, message) }
    pub fn set_cpu_filter_operator_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(31, message) }
    pub fn set_cpu_filter_value_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(32, message) }
    pub fn set_ram_filter_enabled_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(33, message) }
    pub fn set_ram_filter_operator_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(34, message) }
    pub fn set_ram_filter_value_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(35, message) }
    pub fn set_runtime_filter_enabled_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(36, message) }
    pub fn set_runtime_filter_operator_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(37, message) }
    pub fn set_runtime_filter_value_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(38, message) }
    pub fn set_status_filter_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(39, message) }
    pub fn set_process_count_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(40, message) }
    pub fn set_memory_free_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(41, message) }
    pub fn set_network_rx_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(42, message) }
    pub fn set_network_tx_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(43, message) }
    pub fn set_storage_used_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(44, message) }
    pub fn set_storage_total_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(45, message) }
    pub fn set_storage_free_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(46, message) }
    pub fn set_refresh_rate_ms_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(47, message) }
    pub fn set_refresh_rate_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(48, message) }
    pub fn set_memory_used_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(49, message) }
    pub fn set_memory_total_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(50, message) }
    pub fn set_memory_free_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(51, message) }
    pub fn set_network_rx_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(52, message) }
    pub fn set_network_tx_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(53, message) }
    pub fn set_storage_used_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(54, message) }
    pub fn set_storage_total_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(55, message) }
    pub fn set_storage_free_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(56, message) }
    pub fn set_uptime_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(57, message) }
    pub fn set_load_one_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(58, message) }
    pub fn set_load_five_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(59, message) }
    pub fn set_load_fifteen_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(60, message) }
    pub fn set_show_filters_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(61, message) }
    pub fn set_show_columns_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(62, message) }
    pub fn set_show_search_help_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(63, message) }
    pub fn set_show_pid_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(64, message) }
    pub fn set_show_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(65, message) }
    pub fn set_show_user_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(66, message) }
    pub fn set_show_cpu_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(67, message) }
    pub fn set_show_ram_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(68, message) }
    pub fn set_show_virt_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(69, message) }
    pub fn set_show_disk_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(70, message) }
    pub fn set_show_run_time_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(71, message) }
    pub fn set_show_command_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(72, message) }
    pub fn set_show_ppid_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(73, message) }
    pub fn set_show_root_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(74, message) }
    pub fn set_show_environ_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(75, message) }
    pub fn set_show_session_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(76, message) }
    pub fn set_show_start_time_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(77, message) }
    pub fn set_detail_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(78, message) }
    pub fn set_detail_pid_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(79, message) }
    pub fn set_detail_ppid_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(80, message) }
    pub fn set_detail_user_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(81, message) }
    pub fn set_detail_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(82, message) }
    pub fn set_detail_session_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(83, message) }
    pub fn set_detail_cpu_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(84, message) }
    pub fn set_detail_memory_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(85, message) }
    pub fn set_detail_virt_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(86, message) }
    pub fn set_detail_disk_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(87, message) }
    pub fn set_detail_command_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(88, message) }
    pub fn set_detail_root_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(89, message) }
    pub fn set_detail_children_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(90, message) }
    pub fn set_cpu_percent_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(91, message) }
    pub fn set_memory_percent_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(92, message) }
    pub fn set_storage_percent_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(93, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> MainViewModelSinkBatch { MainViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: MainViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct MainViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl MainViewModelSinkBatch {
    pub fn set_search_text(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_search_text_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_search_text_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_is_frozen(&mut self, value: bool) { self.0.push_boolean(3, 2, value); }
    pub fn set_is_frozen_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_is_frozen_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_is_dark_theme(&mut self, value: bool) { self.0.push_boolean(3, 3, value); }
    pub fn set_is_dark_theme_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_is_dark_theme_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_cpu_percent(&mut self, value: f64) { self.0.push_double(4, value); }
    pub fn set_cpu_percent_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_cpu_percent_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_memory_percent(&mut self, value: f64) { self.0.push_double(5, value); }
    pub fn set_memory_percent_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_memory_percent_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_memory_used(&mut self, value: i64) { self.0.push_integer(6, value); }
    pub fn set_memory_used_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_memory_used_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_memory_total(&mut self, value: i64) { self.0.push_integer(7, value); }
    pub fn set_memory_total_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_memory_total_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_process_count(&mut self, value: i64) { self.0.push_integer(8, value); }
    pub fn set_process_count_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_process_count_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 9, 0, value); }
    pub fn set_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_status_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_selected_pid(&mut self, value: i64) { self.0.push_integer(10, value); }
    pub fn set_selected_pid_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_selected_pid_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_selected_index(&mut self, value: i64) { self.0.push_integer(11, value); }
    pub fn set_selected_index_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_selected_index_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_selected_key(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 12, 0, value); }
    pub fn set_selected_key_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_selected_key_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_sort_direction(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 13, 0, value); }
    pub fn set_sort_direction_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_sort_direction_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_error_message(&mut self, value: Option<impl AsRef<str>>) { match value { Some(value) => self.0.push_string(1, 14, 0, value), None => self.0.push_null(14) } }
    pub fn set_error_message_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_error_message_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_cpu_summary(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 15, 0, value); }
    pub fn set_cpu_summary_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_cpu_summary_error(&mut self) { self.0.push_clear_error(15); }
    pub fn set_memory_summary(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 16, 0, value); }
    pub fn set_memory_summary_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 16, 0, message); }
    pub fn clear_memory_summary_error(&mut self) { self.0.push_clear_error(16); }
    pub fn set_network_summary(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 17, 0, value); }
    pub fn set_network_summary_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 17, 0, message); }
    pub fn clear_network_summary_error(&mut self) { self.0.push_clear_error(17); }
    pub fn set_storage_summary(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 18, 0, value); }
    pub fn set_storage_summary_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 18, 0, message); }
    pub fn clear_storage_summary_error(&mut self) { self.0.push_clear_error(18); }
    pub fn set_storage_percent(&mut self, value: f64) { self.0.push_double(19, value); }
    pub fn set_storage_percent_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 19, 0, message); }
    pub fn clear_storage_percent_error(&mut self) { self.0.push_clear_error(19); }
    pub fn set_system_summary(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 20, 0, value); }
    pub fn set_system_summary_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 20, 0, message); }
    pub fn clear_system_summary_error(&mut self) { self.0.push_clear_error(20); }
    pub fn set_freeze_action_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 21, 0, value); }
    pub fn set_freeze_action_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 21, 0, message); }
    pub fn clear_freeze_action_label_error(&mut self) { self.0.push_clear_error(21); }
    pub fn set_theme_action_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 22, 0, value); }
    pub fn set_theme_action_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 22, 0, message); }
    pub fn clear_theme_action_label_error(&mut self) { self.0.push_clear_error(22); }
    pub fn set_show_kill_confirm(&mut self, value: bool) { self.0.push_boolean(3, 23, value); }
    pub fn set_show_kill_confirm_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 23, 0, message); }
    pub fn clear_show_kill_confirm_error(&mut self) { self.0.push_clear_error(23); }
    pub fn set_kill_confirm_message(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 24, 0, value); }
    pub fn set_kill_confirm_message_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 24, 0, message); }
    pub fn clear_kill_confirm_message_error(&mut self) { self.0.push_clear_error(24); }
    pub fn set_is_killing(&mut self, value: bool) { self.0.push_boolean(3, 25, value); }
    pub fn set_is_killing_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 25, 0, message); }
    pub fn clear_is_killing_error(&mut self) { self.0.push_clear_error(25); }
    pub fn set_show_details(&mut self, value: bool) { self.0.push_boolean(3, 26, value); }
    pub fn set_show_details_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 26, 0, message); }
    pub fn clear_show_details_error(&mut self) { self.0.push_clear_error(26); }
    pub fn set_detail_title(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 27, 0, value); }
    pub fn set_detail_title_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 27, 0, message); }
    pub fn clear_detail_title_error(&mut self) { self.0.push_clear_error(27); }
    pub fn set_detail_body(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 28, 0, value); }
    pub fn set_detail_body_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 28, 0, message); }
    pub fn clear_detail_body_error(&mut self) { self.0.push_clear_error(28); }
    pub fn set_show_parent_enabled(&mut self, value: bool) { self.0.push_boolean(3, 29, value); }
    pub fn set_show_parent_enabled_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 29, 0, message); }
    pub fn clear_show_parent_enabled_error(&mut self) { self.0.push_clear_error(29); }
    pub fn set_cpu_filter_enabled(&mut self, value: bool) { self.0.push_boolean(3, 30, value); }
    pub fn set_cpu_filter_enabled_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 30, 0, message); }
    pub fn clear_cpu_filter_enabled_error(&mut self) { self.0.push_clear_error(30); }
    pub fn set_cpu_filter_operator(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 31, 0, value); }
    pub fn set_cpu_filter_operator_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 31, 0, message); }
    pub fn clear_cpu_filter_operator_error(&mut self) { self.0.push_clear_error(31); }
    pub fn set_cpu_filter_value(&mut self, value: i64) { self.0.push_integer(32, value); }
    pub fn set_cpu_filter_value_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 32, 0, message); }
    pub fn clear_cpu_filter_value_error(&mut self) { self.0.push_clear_error(32); }
    pub fn set_ram_filter_enabled(&mut self, value: bool) { self.0.push_boolean(3, 33, value); }
    pub fn set_ram_filter_enabled_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 33, 0, message); }
    pub fn clear_ram_filter_enabled_error(&mut self) { self.0.push_clear_error(33); }
    pub fn set_ram_filter_operator(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 34, 0, value); }
    pub fn set_ram_filter_operator_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 34, 0, message); }
    pub fn clear_ram_filter_operator_error(&mut self) { self.0.push_clear_error(34); }
    pub fn set_ram_filter_value(&mut self, value: i64) { self.0.push_integer(35, value); }
    pub fn set_ram_filter_value_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 35, 0, message); }
    pub fn clear_ram_filter_value_error(&mut self) { self.0.push_clear_error(35); }
    pub fn set_runtime_filter_enabled(&mut self, value: bool) { self.0.push_boolean(3, 36, value); }
    pub fn set_runtime_filter_enabled_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 36, 0, message); }
    pub fn clear_runtime_filter_enabled_error(&mut self) { self.0.push_clear_error(36); }
    pub fn set_runtime_filter_operator(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 37, 0, value); }
    pub fn set_runtime_filter_operator_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 37, 0, message); }
    pub fn clear_runtime_filter_operator_error(&mut self) { self.0.push_clear_error(37); }
    pub fn set_runtime_filter_value(&mut self, value: i64) { self.0.push_integer(38, value); }
    pub fn set_runtime_filter_value_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 38, 0, message); }
    pub fn clear_runtime_filter_value_error(&mut self) { self.0.push_clear_error(38); }
    pub fn set_status_filter(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 39, 0, value); }
    pub fn set_status_filter_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 39, 0, message); }
    pub fn clear_status_filter_error(&mut self) { self.0.push_clear_error(39); }
    pub fn set_process_count_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 40, 0, value); }
    pub fn set_process_count_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 40, 0, message); }
    pub fn clear_process_count_label_error(&mut self) { self.0.push_clear_error(40); }
    pub fn set_memory_free(&mut self, value: i64) { self.0.push_integer(41, value); }
    pub fn set_memory_free_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 41, 0, message); }
    pub fn clear_memory_free_error(&mut self) { self.0.push_clear_error(41); }
    pub fn set_network_rx(&mut self, value: i64) { self.0.push_integer(42, value); }
    pub fn set_network_rx_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 42, 0, message); }
    pub fn clear_network_rx_error(&mut self) { self.0.push_clear_error(42); }
    pub fn set_network_tx(&mut self, value: i64) { self.0.push_integer(43, value); }
    pub fn set_network_tx_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 43, 0, message); }
    pub fn clear_network_tx_error(&mut self) { self.0.push_clear_error(43); }
    pub fn set_storage_used(&mut self, value: i64) { self.0.push_integer(44, value); }
    pub fn set_storage_used_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 44, 0, message); }
    pub fn clear_storage_used_error(&mut self) { self.0.push_clear_error(44); }
    pub fn set_storage_total(&mut self, value: i64) { self.0.push_integer(45, value); }
    pub fn set_storage_total_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 45, 0, message); }
    pub fn clear_storage_total_error(&mut self) { self.0.push_clear_error(45); }
    pub fn set_storage_free(&mut self, value: i64) { self.0.push_integer(46, value); }
    pub fn set_storage_free_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 46, 0, message); }
    pub fn clear_storage_free_error(&mut self) { self.0.push_clear_error(46); }
    pub fn set_refresh_rate_ms(&mut self, value: i64) { self.0.push_integer(47, value); }
    pub fn set_refresh_rate_ms_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 47, 0, message); }
    pub fn clear_refresh_rate_ms_error(&mut self) { self.0.push_clear_error(47); }
    pub fn set_refresh_rate_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 48, 0, value); }
    pub fn set_refresh_rate_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 48, 0, message); }
    pub fn clear_refresh_rate_label_error(&mut self) { self.0.push_clear_error(48); }
    pub fn set_memory_used_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 49, 0, value); }
    pub fn set_memory_used_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 49, 0, message); }
    pub fn clear_memory_used_label_error(&mut self) { self.0.push_clear_error(49); }
    pub fn set_memory_total_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 50, 0, value); }
    pub fn set_memory_total_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 50, 0, message); }
    pub fn clear_memory_total_label_error(&mut self) { self.0.push_clear_error(50); }
    pub fn set_memory_free_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 51, 0, value); }
    pub fn set_memory_free_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 51, 0, message); }
    pub fn clear_memory_free_label_error(&mut self) { self.0.push_clear_error(51); }
    pub fn set_network_rx_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 52, 0, value); }
    pub fn set_network_rx_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 52, 0, message); }
    pub fn clear_network_rx_label_error(&mut self) { self.0.push_clear_error(52); }
    pub fn set_network_tx_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 53, 0, value); }
    pub fn set_network_tx_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 53, 0, message); }
    pub fn clear_network_tx_label_error(&mut self) { self.0.push_clear_error(53); }
    pub fn set_storage_used_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 54, 0, value); }
    pub fn set_storage_used_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 54, 0, message); }
    pub fn clear_storage_used_label_error(&mut self) { self.0.push_clear_error(54); }
    pub fn set_storage_total_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 55, 0, value); }
    pub fn set_storage_total_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 55, 0, message); }
    pub fn clear_storage_total_label_error(&mut self) { self.0.push_clear_error(55); }
    pub fn set_storage_free_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 56, 0, value); }
    pub fn set_storage_free_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 56, 0, message); }
    pub fn clear_storage_free_label_error(&mut self) { self.0.push_clear_error(56); }
    pub fn set_uptime_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 57, 0, value); }
    pub fn set_uptime_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 57, 0, message); }
    pub fn clear_uptime_label_error(&mut self) { self.0.push_clear_error(57); }
    pub fn set_load_one_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 58, 0, value); }
    pub fn set_load_one_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 58, 0, message); }
    pub fn clear_load_one_label_error(&mut self) { self.0.push_clear_error(58); }
    pub fn set_load_five_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 59, 0, value); }
    pub fn set_load_five_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 59, 0, message); }
    pub fn clear_load_five_label_error(&mut self) { self.0.push_clear_error(59); }
    pub fn set_load_fifteen_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 60, 0, value); }
    pub fn set_load_fifteen_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 60, 0, message); }
    pub fn clear_load_fifteen_label_error(&mut self) { self.0.push_clear_error(60); }
    pub fn set_show_filters(&mut self, value: bool) { self.0.push_boolean(3, 61, value); }
    pub fn set_show_filters_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 61, 0, message); }
    pub fn clear_show_filters_error(&mut self) { self.0.push_clear_error(61); }
    pub fn set_show_columns(&mut self, value: bool) { self.0.push_boolean(3, 62, value); }
    pub fn set_show_columns_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 62, 0, message); }
    pub fn clear_show_columns_error(&mut self) { self.0.push_clear_error(62); }
    pub fn set_show_search_help(&mut self, value: bool) { self.0.push_boolean(3, 63, value); }
    pub fn set_show_search_help_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 63, 0, message); }
    pub fn clear_show_search_help_error(&mut self) { self.0.push_clear_error(63); }
    pub fn set_show_pid(&mut self, value: bool) { self.0.push_boolean(3, 64, value); }
    pub fn set_show_pid_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 64, 0, message); }
    pub fn clear_show_pid_error(&mut self) { self.0.push_clear_error(64); }
    pub fn set_show_status(&mut self, value: bool) { self.0.push_boolean(3, 65, value); }
    pub fn set_show_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 65, 0, message); }
    pub fn clear_show_status_error(&mut self) { self.0.push_clear_error(65); }
    pub fn set_show_user(&mut self, value: bool) { self.0.push_boolean(3, 66, value); }
    pub fn set_show_user_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 66, 0, message); }
    pub fn clear_show_user_error(&mut self) { self.0.push_clear_error(66); }
    pub fn set_show_cpu(&mut self, value: bool) { self.0.push_boolean(3, 67, value); }
    pub fn set_show_cpu_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 67, 0, message); }
    pub fn clear_show_cpu_error(&mut self) { self.0.push_clear_error(67); }
    pub fn set_show_ram(&mut self, value: bool) { self.0.push_boolean(3, 68, value); }
    pub fn set_show_ram_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 68, 0, message); }
    pub fn clear_show_ram_error(&mut self) { self.0.push_clear_error(68); }
    pub fn set_show_virt(&mut self, value: bool) { self.0.push_boolean(3, 69, value); }
    pub fn set_show_virt_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 69, 0, message); }
    pub fn clear_show_virt_error(&mut self) { self.0.push_clear_error(69); }
    pub fn set_show_disk(&mut self, value: bool) { self.0.push_boolean(3, 70, value); }
    pub fn set_show_disk_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 70, 0, message); }
    pub fn clear_show_disk_error(&mut self) { self.0.push_clear_error(70); }
    pub fn set_show_run_time(&mut self, value: bool) { self.0.push_boolean(3, 71, value); }
    pub fn set_show_run_time_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 71, 0, message); }
    pub fn clear_show_run_time_error(&mut self) { self.0.push_clear_error(71); }
    pub fn set_show_command(&mut self, value: bool) { self.0.push_boolean(3, 72, value); }
    pub fn set_show_command_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 72, 0, message); }
    pub fn clear_show_command_error(&mut self) { self.0.push_clear_error(72); }
    pub fn set_show_ppid(&mut self, value: bool) { self.0.push_boolean(3, 73, value); }
    pub fn set_show_ppid_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 73, 0, message); }
    pub fn clear_show_ppid_error(&mut self) { self.0.push_clear_error(73); }
    pub fn set_show_root(&mut self, value: bool) { self.0.push_boolean(3, 74, value); }
    pub fn set_show_root_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 74, 0, message); }
    pub fn clear_show_root_error(&mut self) { self.0.push_clear_error(74); }
    pub fn set_show_environ(&mut self, value: bool) { self.0.push_boolean(3, 75, value); }
    pub fn set_show_environ_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 75, 0, message); }
    pub fn clear_show_environ_error(&mut self) { self.0.push_clear_error(75); }
    pub fn set_show_session(&mut self, value: bool) { self.0.push_boolean(3, 76, value); }
    pub fn set_show_session_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 76, 0, message); }
    pub fn clear_show_session_error(&mut self) { self.0.push_clear_error(76); }
    pub fn set_show_start_time(&mut self, value: bool) { self.0.push_boolean(3, 77, value); }
    pub fn set_show_start_time_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 77, 0, message); }
    pub fn clear_show_start_time_error(&mut self) { self.0.push_clear_error(77); }
    pub fn set_detail_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 78, 0, value); }
    pub fn set_detail_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 78, 0, message); }
    pub fn clear_detail_name_error(&mut self) { self.0.push_clear_error(78); }
    pub fn set_detail_pid_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 79, 0, value); }
    pub fn set_detail_pid_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 79, 0, message); }
    pub fn clear_detail_pid_label_error(&mut self) { self.0.push_clear_error(79); }
    pub fn set_detail_ppid_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 80, 0, value); }
    pub fn set_detail_ppid_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 80, 0, message); }
    pub fn clear_detail_ppid_label_error(&mut self) { self.0.push_clear_error(80); }
    pub fn set_detail_user(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 81, 0, value); }
    pub fn set_detail_user_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 81, 0, message); }
    pub fn clear_detail_user_error(&mut self) { self.0.push_clear_error(81); }
    pub fn set_detail_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 82, 0, value); }
    pub fn set_detail_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 82, 0, message); }
    pub fn clear_detail_status_error(&mut self) { self.0.push_clear_error(82); }
    pub fn set_detail_session_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 83, 0, value); }
    pub fn set_detail_session_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 83, 0, message); }
    pub fn clear_detail_session_label_error(&mut self) { self.0.push_clear_error(83); }
    pub fn set_detail_cpu_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 84, 0, value); }
    pub fn set_detail_cpu_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 84, 0, message); }
    pub fn clear_detail_cpu_label_error(&mut self) { self.0.push_clear_error(84); }
    pub fn set_detail_memory_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 85, 0, value); }
    pub fn set_detail_memory_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 85, 0, message); }
    pub fn clear_detail_memory_label_error(&mut self) { self.0.push_clear_error(85); }
    pub fn set_detail_virt_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 86, 0, value); }
    pub fn set_detail_virt_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 86, 0, message); }
    pub fn clear_detail_virt_label_error(&mut self) { self.0.push_clear_error(86); }
    pub fn set_detail_disk_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 87, 0, value); }
    pub fn set_detail_disk_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 87, 0, message); }
    pub fn clear_detail_disk_label_error(&mut self) { self.0.push_clear_error(87); }
    pub fn set_detail_command(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 88, 0, value); }
    pub fn set_detail_command_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 88, 0, message); }
    pub fn clear_detail_command_error(&mut self) { self.0.push_clear_error(88); }
    pub fn set_detail_root(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 89, 0, value); }
    pub fn set_detail_root_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 89, 0, message); }
    pub fn clear_detail_root_error(&mut self) { self.0.push_clear_error(89); }
    pub fn set_detail_children(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 90, 0, value); }
    pub fn set_detail_children_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 90, 0, message); }
    pub fn clear_detail_children_error(&mut self) { self.0.push_clear_error(90); }
    pub fn set_cpu_percent_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 91, 0, value); }
    pub fn set_cpu_percent_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 91, 0, message); }
    pub fn clear_cpu_percent_label_error(&mut self) { self.0.push_clear_error(91); }
    pub fn set_memory_percent_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 92, 0, value); }
    pub fn set_memory_percent_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 92, 0, message); }
    pub fn clear_memory_percent_label_error(&mut self) { self.0.push_clear_error(92); }
    pub fn set_storage_percent_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 93, 0, value); }
    pub fn set_storage_percent_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 93, 0, message); }
    pub fn clear_storage_percent_label_error(&mut self) { self.0.push_clear_error(93); }
    pub fn add_processes(&mut self, value: impl ProcessRowViewModel) { self.0.push_model(8, 1, 0, ProcessRowViewModelDispatch { model: value }); }
    pub fn insert_processes(&mut self, index: i32, value: impl ProcessRowViewModel) { self.0.push_model(10, 1, index, ProcessRowViewModelDispatch { model: value }); }
    pub fn replace_processes(&mut self, index: i32, value: impl ProcessRowViewModel) { self.0.push_model(12, 1, index, ProcessRowViewModelDispatch { model: value }); }
    pub fn replace_processes_snapshot<M: ProcessRowViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(1, values.into_iter().map(|value| ProcessRowViewModelDispatch { model: value })); }
    pub fn remove_processes(&mut self, index: i32) { self.0.push_model_indices(13, 1, index, 0); }
    pub fn move_processes(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 1, from_index, to_index); }
    pub fn clear_processes(&mut self) { self.0.push_model_clear(1); }
    pub fn add_cpu_cores(&mut self, value: impl CpuCoreViewModel) { self.0.push_model(8, 2, 0, CpuCoreViewModelDispatch { model: value }); }
    pub fn insert_cpu_cores(&mut self, index: i32, value: impl CpuCoreViewModel) { self.0.push_model(10, 2, index, CpuCoreViewModelDispatch { model: value }); }
    pub fn replace_cpu_cores(&mut self, index: i32, value: impl CpuCoreViewModel) { self.0.push_model(12, 2, index, CpuCoreViewModelDispatch { model: value }); }
    pub fn replace_cpu_cores_snapshot<M: CpuCoreViewModel>(&mut self, values: impl IntoIterator<Item = M>) { self.0.push_model_snapshot(2, values.into_iter().map(|value| CpuCoreViewModelDispatch { model: value })); }
    pub fn remove_cpu_cores(&mut self, index: i32) { self.0.push_model_indices(13, 2, index, 0); }
    pub fn move_cpu_cores(&mut self, from_index: i32, to_index: i32) { self.0.push_model_indices(14, 2, from_index, to_index); }
    pub fn clear_cpu_cores(&mut self) { self.0.push_model_clear(2); }
    pub fn set_refresh_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 1, enabled); }
    pub fn set_kill_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 2, enabled); }
    pub fn set_pin_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 3, enabled); }
    pub fn set_show_details_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 4, enabled); }
    pub fn set_toggle_theme_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 5, enabled); }
    pub fn set_toggle_freeze_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 6, enabled); }
    pub fn set_sort_processes_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 7, enabled); }
    pub fn set_confirm_kill_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 8, enabled); }
    pub fn set_cancel_kill_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 9, enabled); }
    pub fn set_close_details_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 10, enabled); }
    pub fn set_show_parent_details_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 11, enabled); }
    pub fn set_copy_selected_row_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 12, enabled); }
    pub fn set_exit_application_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 13, enabled); }
    pub fn set_clear_search_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 14, enabled); }
    pub fn set_toggle_cpu_operator_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 15, enabled); }
    pub fn set_toggle_ram_operator_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 16, enabled); }
    pub fn set_toggle_runtime_operator_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 17, enabled); }
    pub fn set_toggle_filters_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 18, enabled); }
    pub fn set_toggle_columns_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 19, enabled); }
    pub fn set_toggle_search_help_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 20, enabled); }
    pub fn set_cycle_refresh_rate_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 21, enabled); }
    pub fn set_close_overlays_enabled(&mut self, enabled: bool) { self.0.push_boolean(17, 22, enabled); }
}

pub trait MainViewModel: Send + 'static {
    fn attach(&mut self, sink: MainViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn set_search_text(&mut self, value: String) -> crate::Result<()>;
    fn set_is_frozen(&mut self, value: bool) -> crate::Result<()>;
    fn set_is_dark_theme(&mut self, value: bool) -> crate::Result<()>;
    fn set_selected_pid(&mut self, value: i64) -> crate::Result<()>;
    fn set_selected_index(&mut self, value: i64) -> crate::Result<()>;
    fn set_selected_key(&mut self, value: String) -> crate::Result<()>;
    fn set_cpu_filter_enabled(&mut self, value: bool) -> crate::Result<()>;
    fn set_cpu_filter_operator(&mut self, value: String) -> crate::Result<()>;
    fn set_cpu_filter_value(&mut self, value: i64) -> crate::Result<()>;
    fn set_ram_filter_enabled(&mut self, value: bool) -> crate::Result<()>;
    fn set_ram_filter_operator(&mut self, value: String) -> crate::Result<()>;
    fn set_ram_filter_value(&mut self, value: i64) -> crate::Result<()>;
    fn set_runtime_filter_enabled(&mut self, value: bool) -> crate::Result<()>;
    fn set_runtime_filter_operator(&mut self, value: String) -> crate::Result<()>;
    fn set_runtime_filter_value(&mut self, value: i64) -> crate::Result<()>;
    fn set_status_filter(&mut self, value: String) -> crate::Result<()>;
    fn set_refresh_rate_ms(&mut self, value: i64) -> crate::Result<()>;
    fn set_show_filters(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_columns(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_search_help(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_pid(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_status(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_user(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_cpu(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_ram(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_virt(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_disk(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_run_time(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_command(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_ppid(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_root(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_environ(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_session(&mut self, value: bool) -> crate::Result<()>;
    fn set_show_start_time(&mut self, value: bool) -> crate::Result<()>;
    fn refresh(&mut self) -> crate::Result<()>;
    fn kill(&mut self, value: String) -> crate::Result<()>;
    fn pin(&mut self, value: String) -> crate::Result<()>;
    fn show_details(&mut self, value: String) -> crate::Result<()>;
    fn toggle_theme(&mut self) -> crate::Result<()>;
    fn toggle_freeze(&mut self) -> crate::Result<()>;
    fn sort_processes(&mut self, value: String) -> crate::Result<()>;
    fn confirm_kill(&mut self) -> crate::Result<()>;
    fn cancel_kill(&mut self) -> crate::Result<()>;
    fn close_details(&mut self) -> crate::Result<()>;
    fn show_parent_details(&mut self) -> crate::Result<()>;
    fn copy_selected_row(&mut self) -> crate::Result<()>;
    fn exit_application(&mut self) -> crate::Result<()>;
    fn clear_search(&mut self) -> crate::Result<()>;
    fn toggle_cpu_operator(&mut self) -> crate::Result<()>;
    fn toggle_ram_operator(&mut self) -> crate::Result<()>;
    fn toggle_runtime_operator(&mut self) -> crate::Result<()>;
    fn toggle_filters(&mut self) -> crate::Result<()>;
    fn toggle_columns(&mut self) -> crate::Result<()>;
    fn toggle_search_help(&mut self) -> crate::Result<()>;
    fn cycle_refresh_rate(&mut self) -> crate::Result<()>;
    fn close_overlays(&mut self) -> crate::Result<()>;
}

struct MainViewModelDispatch<T: MainViewModel> { model: T }

impl<T: MainViewModel> crate::view_model::DynamicViewModel for MainViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(MainViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, value: String) -> crate::Result<()> {
        match property_id {
            1 => self.model.set_search_text(value),
            12 => self.model.set_selected_key(value),
            31 => self.model.set_cpu_filter_operator(value),
            34 => self.model.set_ram_filter_operator(value),
            37 => self.model.set_runtime_filter_operator(value),
            39 => self.model.set_status_filter(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_integer(&mut self, property_id: i32, value: i64) -> crate::Result<()> {
        match property_id {
            10 => self.model.set_selected_pid(value),
            11 => self.model.set_selected_index(value),
            32 => self.model.set_cpu_filter_value(value),
            35 => self.model.set_ram_filter_value(value),
            38 => self.model.set_runtime_filter_value(value),
            47 => self.model.set_refresh_rate_ms(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_boolean(&mut self, property_id: i32, value: bool) -> crate::Result<()> {
        match property_id {
            2 => self.model.set_is_frozen(value),
            3 => self.model.set_is_dark_theme(value),
            30 => self.model.set_cpu_filter_enabled(value),
            33 => self.model.set_ram_filter_enabled(value),
            36 => self.model.set_runtime_filter_enabled(value),
            61 => self.model.set_show_filters(value),
            62 => self.model.set_show_columns(value),
            63 => self.model.set_show_search_help(value),
            64 => self.model.set_show_pid(value),
            65 => self.model.set_show_status(value),
            66 => self.model.set_show_user(value),
            67 => self.model.set_show_cpu(value),
            68 => self.model.set_show_ram(value),
            69 => self.model.set_show_virt(value),
            70 => self.model.set_show_disk(value),
            71 => self.model.set_show_run_time(value),
            72 => self.model.set_show_command(value),
            73 => self.model.set_show_ppid(value),
            74 => self.model.set_show_root(value),
            75 => self.model.set_show_environ(value),
            76 => self.model.set_show_session(value),
            77 => self.model.set_show_start_time(value),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id }),
        }
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            1 => self.model.refresh(),
            3 => self.model.pin(parameter.unwrap_or_default()),
            4 => self.model.show_details(parameter.unwrap_or_default()),
            5 => self.model.toggle_theme(),
            6 => self.model.toggle_freeze(),
            7 => self.model.sort_processes(parameter.unwrap_or_default()),
            8 => self.model.confirm_kill(),
            9 => self.model.cancel_kill(),
            10 => self.model.close_details(),
            11 => self.model.show_parent_details(),
            13 => self.model.exit_application(),
            14 => self.model.clear_search(),
            15 => self.model.toggle_cpu_operator(),
            16 => self.model.toggle_ram_operator(),
            17 => self.model.toggle_runtime_operator(),
            18 => self.model.toggle_filters(),
            19 => self.model.toggle_columns(),
            20 => self.model.toggle_search_help(),
            21 => self.model.cycle_refresh_rate(),
            22 => self.model.close_overlays(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
    fn begin_async(&mut self, command_id: i32, parameter: Option<String>) -> crate::Result<()> {
        match command_id {
            2 => self.model.kill(parameter.unwrap_or_default()),
            12 => self.model.copy_selected_row(),
            _ => Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id }),
        }
    }
}

pub fn mount_main_window(scope: &crate::AppScope, model: impl MainViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model(1, MainViewModelDispatch { model }) }

#[derive(Clone, Debug)]
pub struct ProcessRowViewModelSink(crate::view_model::ViewModelSink);

impl ProcessRowViewModelSink {
    pub fn set_name(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_pid(&self, value: i64) -> crate::Result<()> { self.0.set_integer(2, value) }
    pub fn set_ppid(&self, value: i64) -> crate::Result<()> { self.0.set_integer(3, value) }
    pub fn set_status(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(4, value) }
    pub fn set_user(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(5, value) }
    pub fn set_cpu_usage(&self, value: f64) -> crate::Result<()> { self.0.set_double(6, value) }
    pub fn set_memory_usage(&self, value: i64) -> crate::Result<()> { self.0.set_integer(7, value) }
    pub fn set_virtual_memory(&self, value: i64) -> crate::Result<()> { self.0.set_integer(8, value) }
    pub fn set_disk_read(&self, value: i64) -> crate::Result<()> { self.0.set_integer(9, value) }
    pub fn set_disk_write(&self, value: i64) -> crate::Result<()> { self.0.set_integer(10, value) }
    pub fn set_command(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(11, value) }
    pub fn set_root(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(12, value) }
    pub fn set_session_id(&self, value: i64) -> crate::Result<()> { self.0.set_integer(13, value) }
    pub fn set_start_time(&self, value: i64) -> crate::Result<()> { self.0.set_integer(14, value) }
    pub fn set_run_time(&self, value: i64) -> crate::Result<()> { self.0.set_integer(15, value) }
    pub fn set_is_pinned(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(16, value) }
    pub fn set_key(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(17, value) }
    pub fn set_environ(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(18, value) }
    pub fn set_disk_io(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(19, value) }
    pub fn set_run_time_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(20, value) }
    pub fn set_memory_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(21, value) }
    pub fn set_virtual_memory_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(22, value) }
    pub fn set_cpu_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(23, value) }
    pub fn set_pin_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(24, value) }
    pub fn set_is_high_usage(&self, value: bool) -> crate::Result<()> { self.0.set_boolean(25, value) }
    pub fn set_start_time_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(26, value) }
    pub fn set_session_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(27, value) }
    pub fn set_icon_png(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(28, value) }
    pub fn set_name_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_pid_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    pub fn set_ppid_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(3, message) }
    pub fn set_status_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(4, message) }
    pub fn set_user_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(5, message) }
    pub fn set_cpu_usage_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(6, message) }
    pub fn set_memory_usage_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(7, message) }
    pub fn set_virtual_memory_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(8, message) }
    pub fn set_disk_read_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(9, message) }
    pub fn set_disk_write_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(10, message) }
    pub fn set_command_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(11, message) }
    pub fn set_root_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(12, message) }
    pub fn set_session_id_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(13, message) }
    pub fn set_start_time_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(14, message) }
    pub fn set_run_time_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(15, message) }
    pub fn set_is_pinned_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(16, message) }
    pub fn set_key_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(17, message) }
    pub fn set_environ_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(18, message) }
    pub fn set_disk_io_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(19, message) }
    pub fn set_run_time_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(20, message) }
    pub fn set_memory_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(21, message) }
    pub fn set_virtual_memory_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(22, message) }
    pub fn set_cpu_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(23, message) }
    pub fn set_pin_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(24, message) }
    pub fn set_is_high_usage_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(25, message) }
    pub fn set_start_time_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(26, message) }
    pub fn set_session_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(27, message) }
    pub fn set_icon_png_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(28, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> ProcessRowViewModelSinkBatch { ProcessRowViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: ProcessRowViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct ProcessRowViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl ProcessRowViewModelSinkBatch {
    pub fn set_name(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_name_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_name_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_pid(&mut self, value: i64) { self.0.push_integer(2, value); }
    pub fn set_pid_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_pid_error(&mut self) { self.0.push_clear_error(2); }
    pub fn set_ppid(&mut self, value: i64) { self.0.push_integer(3, value); }
    pub fn set_ppid_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 3, 0, message); }
    pub fn clear_ppid_error(&mut self) { self.0.push_clear_error(3); }
    pub fn set_status(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 4, 0, value); }
    pub fn set_status_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 4, 0, message); }
    pub fn clear_status_error(&mut self) { self.0.push_clear_error(4); }
    pub fn set_user(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 5, 0, value); }
    pub fn set_user_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 5, 0, message); }
    pub fn clear_user_error(&mut self) { self.0.push_clear_error(5); }
    pub fn set_cpu_usage(&mut self, value: f64) { self.0.push_double(6, value); }
    pub fn set_cpu_usage_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 6, 0, message); }
    pub fn clear_cpu_usage_error(&mut self) { self.0.push_clear_error(6); }
    pub fn set_memory_usage(&mut self, value: i64) { self.0.push_integer(7, value); }
    pub fn set_memory_usage_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 7, 0, message); }
    pub fn clear_memory_usage_error(&mut self) { self.0.push_clear_error(7); }
    pub fn set_virtual_memory(&mut self, value: i64) { self.0.push_integer(8, value); }
    pub fn set_virtual_memory_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 8, 0, message); }
    pub fn clear_virtual_memory_error(&mut self) { self.0.push_clear_error(8); }
    pub fn set_disk_read(&mut self, value: i64) { self.0.push_integer(9, value); }
    pub fn set_disk_read_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 9, 0, message); }
    pub fn clear_disk_read_error(&mut self) { self.0.push_clear_error(9); }
    pub fn set_disk_write(&mut self, value: i64) { self.0.push_integer(10, value); }
    pub fn set_disk_write_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 10, 0, message); }
    pub fn clear_disk_write_error(&mut self) { self.0.push_clear_error(10); }
    pub fn set_command(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 11, 0, value); }
    pub fn set_command_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 11, 0, message); }
    pub fn clear_command_error(&mut self) { self.0.push_clear_error(11); }
    pub fn set_root(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 12, 0, value); }
    pub fn set_root_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 12, 0, message); }
    pub fn clear_root_error(&mut self) { self.0.push_clear_error(12); }
    pub fn set_session_id(&mut self, value: i64) { self.0.push_integer(13, value); }
    pub fn set_session_id_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 13, 0, message); }
    pub fn clear_session_id_error(&mut self) { self.0.push_clear_error(13); }
    pub fn set_start_time(&mut self, value: i64) { self.0.push_integer(14, value); }
    pub fn set_start_time_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 14, 0, message); }
    pub fn clear_start_time_error(&mut self) { self.0.push_clear_error(14); }
    pub fn set_run_time(&mut self, value: i64) { self.0.push_integer(15, value); }
    pub fn set_run_time_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 15, 0, message); }
    pub fn clear_run_time_error(&mut self) { self.0.push_clear_error(15); }
    pub fn set_is_pinned(&mut self, value: bool) { self.0.push_boolean(3, 16, value); }
    pub fn set_is_pinned_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 16, 0, message); }
    pub fn clear_is_pinned_error(&mut self) { self.0.push_clear_error(16); }
    pub fn set_key(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 17, 0, value); }
    pub fn set_key_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 17, 0, message); }
    pub fn clear_key_error(&mut self) { self.0.push_clear_error(17); }
    pub fn set_environ(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 18, 0, value); }
    pub fn set_environ_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 18, 0, message); }
    pub fn clear_environ_error(&mut self) { self.0.push_clear_error(18); }
    pub fn set_disk_io(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 19, 0, value); }
    pub fn set_disk_io_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 19, 0, message); }
    pub fn clear_disk_io_error(&mut self) { self.0.push_clear_error(19); }
    pub fn set_run_time_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 20, 0, value); }
    pub fn set_run_time_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 20, 0, message); }
    pub fn clear_run_time_label_error(&mut self) { self.0.push_clear_error(20); }
    pub fn set_memory_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 21, 0, value); }
    pub fn set_memory_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 21, 0, message); }
    pub fn clear_memory_label_error(&mut self) { self.0.push_clear_error(21); }
    pub fn set_virtual_memory_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 22, 0, value); }
    pub fn set_virtual_memory_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 22, 0, message); }
    pub fn clear_virtual_memory_label_error(&mut self) { self.0.push_clear_error(22); }
    pub fn set_cpu_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 23, 0, value); }
    pub fn set_cpu_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 23, 0, message); }
    pub fn clear_cpu_label_error(&mut self) { self.0.push_clear_error(23); }
    pub fn set_pin_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 24, 0, value); }
    pub fn set_pin_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 24, 0, message); }
    pub fn clear_pin_label_error(&mut self) { self.0.push_clear_error(24); }
    pub fn set_is_high_usage(&mut self, value: bool) { self.0.push_boolean(3, 25, value); }
    pub fn set_is_high_usage_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 25, 0, message); }
    pub fn clear_is_high_usage_error(&mut self) { self.0.push_clear_error(25); }
    pub fn set_start_time_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 26, 0, value); }
    pub fn set_start_time_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 26, 0, message); }
    pub fn clear_start_time_label_error(&mut self) { self.0.push_clear_error(26); }
    pub fn set_session_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 27, 0, value); }
    pub fn set_session_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 27, 0, message); }
    pub fn clear_session_label_error(&mut self) { self.0.push_clear_error(27); }
    pub fn set_icon_png(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 28, 0, value); }
    pub fn set_icon_png_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 28, 0, message); }
    pub fn clear_icon_png_error(&mut self) { self.0.push_clear_error(28); }
}

pub trait ProcessRowViewModel: Send + 'static {
    fn attach(&mut self, sink: ProcessRowViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
}

struct ProcessRowViewModelDispatch<T: ProcessRowViewModel> { model: T }

impl<T: ProcessRowViewModel> crate::view_model::DynamicViewModel for ProcessRowViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(ProcessRowViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, _value: String) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, _value: bool) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}


#[derive(Clone, Debug)]
pub struct CpuCoreViewModelSink(crate::view_model::ViewModelSink);

impl CpuCoreViewModelSink {
    pub fn set_label(&self, value: impl AsRef<str>) -> crate::Result<()> { self.0.set_string(1, value) }
    pub fn set_usage(&self, value: f64) -> crate::Result<()> { self.0.set_double(2, value) }
    pub fn set_label_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(1, message) }
    pub fn set_usage_error(&self, message: Option<&str>) -> crate::Result<()> { self.0.set_property_error(2, message) }
    /// Creates a worker-safe immutable update batch with a monotonic generation.
    pub fn batch(&self, generation: i64) -> CpuCoreViewModelSinkBatch { CpuCoreViewModelSinkBatch(crate::view_model::ViewModelBatch::new(generation)) }
    pub fn submit_batch(&self, batch: CpuCoreViewModelSinkBatch) -> crate::Result<crate::view_model::BatchCompletion> { self.0.submit_batch(batch.0) }
}

pub struct CpuCoreViewModelSinkBatch(crate::view_model::ViewModelBatch);

impl CpuCoreViewModelSinkBatch {
    pub fn set_label(&mut self, value: impl AsRef<str>) { self.0.push_string(1, 1, 0, value); }
    pub fn set_label_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 1, 0, message); }
    pub fn clear_label_error(&mut self) { self.0.push_clear_error(1); }
    pub fn set_usage(&mut self, value: f64) { self.0.push_double(2, value); }
    pub fn set_usage_error(&mut self, message: impl AsRef<str>) { self.0.push_string(18, 2, 0, message); }
    pub fn clear_usage_error(&mut self) { self.0.push_clear_error(2); }
}

pub trait CpuCoreViewModel: Send + 'static {
    fn attach(&mut self, sink: CpuCoreViewModelSink) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
}

struct CpuCoreViewModelDispatch<T: CpuCoreViewModel> { model: T }

impl<T: CpuCoreViewModel> crate::view_model::DynamicViewModel for CpuCoreViewModelDispatch<T> {
    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> { self.model.attach(CpuCoreViewModelSink(sink)) }
    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }
    fn set_string(&mut self, property_id: i32, _value: String) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_integer(&mut self, property_id: i32, _value: i64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_boolean(&mut self, property_id: i32, _value: bool) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn set_double(&mut self, property_id: i32, _value: f64) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "property", id: property_id })
    }
    fn execute(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
    fn begin_async(&mut self, command_id: i32, _parameter: Option<String>) -> crate::Result<()> {
        Err(crate::Error::InvalidViewModelMember { kind: "command", id: command_id })
    }
}
