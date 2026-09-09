use avalonia_sys::{
    AvnOptionalDateTime, AvnOptionalTimeSpan, DOTNET_EPOCH_OFFSET_TICKS, DOTNET_MAX_DATE_TIME_TICKS,
};
use std::time::{Duration, UNIX_EPOCH};

#[test]
fn dates_roundtrip_across_epoch_and_dotnet_bounds() {
    for ticks in [
        0,
        1,
        DOTNET_EPOCH_OFFSET_TICKS - 10_000_001,
        DOTNET_EPOCH_OFFSET_TICKS - 1,
        DOTNET_EPOCH_OFFSET_TICKS,
        DOTNET_EPOCH_OFFSET_TICKS + 1,
        DOTNET_MAX_DATE_TIME_TICKS,
    ] {
        let value = AvnOptionalDateTime {
            has_value: 1,
            ticks,
        };
        let delta_ticks = (ticks - DOTNET_EPOCH_OFFSET_TICKS).unsigned_abs();
        let delta = Duration::new(
            delta_ticks / 10_000_000,
            (delta_ticks % 10_000_000) as u32 * 100,
        );
        let expected = if ticks < DOTNET_EPOCH_OFFSET_TICKS {
            UNIX_EPOCH.checked_sub(delta)
        } else {
            UNIX_EPOCH.checked_add(delta)
        };
        let Some(time) = expected else {
            assert!(
                value.try_to_date_time().is_err(),
                "unrepresentable platform time"
            );
            continue;
        };
        assert_eq!(value.try_to_date_time().unwrap(), Some(time));
        assert_eq!(
            AvnOptionalDateTime::try_from_date_time(Some(time)).unwrap(),
            value
        );
    }
    let before = UNIX_EPOCH - Duration::new(1, 100);
    assert_eq!(
        AvnOptionalDateTime::from_date_time(Some(before)).ticks,
        DOTNET_EPOCH_OFFSET_TICKS - 10_000_001
    );
}

#[test]
fn dates_reject_out_of_range_values() {
    for ticks in [-1, DOTNET_MAX_DATE_TIME_TICKS + 1, i64::MIN, i64::MAX] {
        assert!(AvnOptionalDateTime {
            has_value: 1,
            ticks
        }
        .try_to_date_time()
        .is_err());
    }
    if let Ok(Some(min)) = (AvnOptionalDateTime {
        has_value: 1,
        ticks: 0,
    })
    .try_to_date_time()
    {
        if let Some(before_min) = min.checked_sub(Duration::from_nanos(100)) {
            assert!(AvnOptionalDateTime::try_from_date_time(Some(before_min)).is_err());
        }
    }
    let max = AvnOptionalDateTime {
        has_value: 1,
        ticks: DOTNET_MAX_DATE_TIME_TICKS,
    }
    .to_date_time()
    .unwrap();
    assert!(
        AvnOptionalDateTime::try_from_date_time(max.checked_add(Duration::from_nanos(100)))
            .is_err()
    );
}

#[test]
fn dates_truncate_subticks_toward_unix_epoch() {
    let mut times = vec![std::time::SystemTime::now()];
    for nanos in [1, 99, 100, 101, 199, 1_000_000_099] {
        times.push(UNIX_EPOCH + Duration::from_nanos(nanos));
        times.push(UNIX_EPOCH - Duration::from_nanos(nanos));
    }
    for time in times {
        // Derive the expected value from the actual SystemTime: Windows already
        // quantizes to 100ns, whereas Unix platforms retain nanosecond precision.
        let unix_ticks = match time.duration_since(UNIX_EPOCH) {
            Ok(delta) => (delta.as_nanos() / 100) as i64,
            Err(error) => -((error.duration().as_nanos() / 100) as i64),
        };
        let expected = AvnOptionalDateTime {
            has_value: 1,
            ticks: DOTNET_EPOCH_OFFSET_TICKS + unix_ticks,
        };
        assert_eq!(
            AvnOptionalDateTime::try_from_date_time(Some(time)).unwrap(),
            expected
        );
        assert_eq!(AvnOptionalDateTime::from_date_time(Some(time)), expected);
    }
}

#[test]
fn durations_roundtrip_without_nanosecond_overflow() {
    for ticks in [0, 1, 10_000_001, 100_000_000_000_000_000, i64::MAX] {
        let value = AvnOptionalTimeSpan {
            has_value: 1,
            ticks,
        };
        let duration = value.try_to_duration().unwrap().unwrap();
        assert_eq!(duration.as_secs(), ticks as u64 / 10_000_000);
        assert_eq!(
            duration.subsec_nanos(),
            (ticks as u64 % 10_000_000) as u32 * 100
        );
        assert_eq!(
            AvnOptionalTimeSpan::try_from_duration(Some(duration)).unwrap(),
            value
        );
    }
}

#[test]
fn durations_reject_negative_and_overflow_values() {
    for ticks in [-1, i64::MIN] {
        assert!(AvnOptionalTimeSpan {
            has_value: 1,
            ticks
        }
        .try_to_duration()
        .is_err());
    }
    let max = AvnOptionalTimeSpan {
        has_value: 1,
        ticks: i64::MAX,
    }
    .to_duration()
    .unwrap();
    for duration in [max + Duration::from_nanos(100), Duration::MAX] {
        assert!(AvnOptionalTimeSpan::try_from_duration(Some(duration)).is_err());
    }

    assert_eq!(AvnOptionalTimeSpan::from_duration(None).to_duration(), None);
    assert_eq!(
        AvnOptionalDateTime::from_date_time(None).to_date_time(),
        None
    );
}

#[test]
fn durations_truncate_subticks_toward_zero() {
    for nanos in [1, 99, 100, 101, 199, 1_000_000_099] {
        let duration = Duration::from_nanos(nanos);
        let expected = AvnOptionalTimeSpan {
            has_value: 1,
            ticks: (nanos / 100) as i64,
        };
        assert_eq!(
            AvnOptionalTimeSpan::try_from_duration(Some(duration)).unwrap(),
            expected
        );
        assert_eq!(AvnOptionalTimeSpan::from_duration(Some(duration)), expected);
    }
}
