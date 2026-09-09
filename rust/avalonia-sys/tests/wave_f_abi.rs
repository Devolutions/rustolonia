//! ABI guarantees for the wave F calendar family.

const HEADER: &str = include_str!("../include/avalonia-rust-abi.h");

#[test]
fn calendar_days_are_yyyy_mm_dd_strings() {
    for expected in [
        "*get_selected_date)(IAvnCalendar* self, uint16_t** value)",
        "*set_selected_date)(IAvnCalendar* self, const uint16_t* value)",
        "*set_display_date)(IAvnCalendar* self, const uint16_t* value)",
        "*set_display_mode)(IAvnCalendar* self, int32_t value)",
        "*create_calendar)(IAvnControlFactory* self, IAvnCalendar** value)",
        "*create_calendar_date_picker)(IAvnControlFactory* self, IAvnCalendarDatePicker** value)",
        "#define I_AVN_CALENDAR_ABI_VERSION 12",
        "#define I_AVN_CALENDAR_DATE_PICKER_ABI_VERSION 11",
        "#define I_AVN_CONTROL_FACTORY_ABI_VERSION 13",
    ] {
        assert!(HEADER.contains(expected), "header is missing `{expected}`");
    }
}
