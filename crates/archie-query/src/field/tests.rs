use super::{Date, DateTime};

#[test]
fn a_date_must_be_a_real_calendar_day() {
    for text in [
        "2031-02-31",
        "2031-04-31",
        "2031-02-29",
        "2100-02-29",
        "2031-00-10",
        "2031-13-01",
        "2031-05-00",
        "+031-05-01",
        "2031-+5-01",
        "2031-05-+1",
    ] {
        assert!(Date::parse(text).is_none(), "{text}");
    }
    for text in ["2032-02-29", "2000-02-29", "2031-04-30", "2031-12-31"] {
        assert!(Date::parse(text).is_some(), "{text}");
    }
}

#[test]
fn a_time_must_be_a_real_time_of_day() {
    for text in [
        "2031-05-01T25:00:00",
        "2031-05-01T24:00:00",
        "2031-05-01T23:60:00",
        "2031-05-01T23:59:60",
        "2031-05-01T+1:00:00",
        "2031-05-01T10:00:00.Z",
    ] {
        assert!(DateTime::parse(text).is_none(), "{text}");
    }
    assert!(DateTime::parse("2031-05-01T23:59:59").is_some());
}

#[test]
fn an_offset_must_be_a_real_offset() {
    for text in [
        "2031-05-01T10:00:00+24:00",
        "2031-05-01T10:00:00+05:60",
        "2031-05-01T10:00:00+-5:00",
        "2031-05-01T10:00:00++5:00",
    ] {
        assert!(DateTime::parse(text).is_none(), "{text}");
    }
    assert!(DateTime::parse("2031-05-01T10:00:00+23:59").is_some());
    assert!(DateTime::parse("2031-05-01T10:00:00-04:00").is_some());
}
