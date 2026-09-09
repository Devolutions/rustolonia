#![allow(dead_code)]
use std::cell::RefCell;

type Result<T> = std::result::Result<T, &'static str>;

trait AsControl {
    fn as_control(&self) -> Result<u32>;
}

struct Control;
impl AsControl for Control {
    fn as_control(&self) -> Result<u32> { Ok(42) }
}

struct FailingControl;
impl AsControl for FailingControl {
    fn as_control(&self) -> Result<u32> { Err("query failed") }
}

struct Specific { raw: u32 }

#[derive(Default)]
struct Raw {
    calls: RefCell<Vec<Option<u32>>>,
    strings: RefCell<Vec<(Vec<u16>, Option<Vec<u16>>)>>,
}

impl Raw {
    fn set_required_control(&self, value: &u32) -> Result<()> {
        self.set_optional_control(Some(value))
    }
    fn set_optional_control(&self, value: Option<&u32>) -> Result<()> {
        self.calls.borrow_mut().push(value.copied());
        Ok(())
    }
    fn set_required_specific(&self, value: &u32) -> Result<()> {
        self.set_optional_control(Some(value))
    }
    fn set_optional_specific(&self, value: Option<&u32>) -> Result<()> {
        self.set_optional_control(value)
    }
    fn write_strings(&self, required: &[u16], optional: Option<&[u16]>) -> Result<()> {
        self.strings.borrow_mut().push((required.to_vec(), optional.map(<[u16]>::to_vec)));
        Ok(())
    }
}

#[derive(Default)]
struct Fixture { raw: Raw }

impl Fixture {
    // GENERATED_MEMBERS
}

fn main() -> Result<()> {
    let fixture = Fixture::default()
        .required_control(Control)?
        .optional_control(Some(&Control))?
        .optional_control(None)?
        .required_specific(&Specific { raw: 7 })?
        .optional_specific(Some(&Specific { raw: 8 }))?
        .optional_specific(None)?;
    fixture.set_required_control(Control)?;
    fixture.set_optional_control(Some(&Control))?;
    fixture.set_optional_control(None)?;
    fixture.set_required_specific(&Specific { raw: 7 })?;
    fixture.set_optional_specific(Some(&Specific { raw: 8 }))?;
    fixture.set_optional_specific(None)?;
    assert_eq!(*fixture.raw.calls.borrow(),
        [Some(42), Some(42), None, Some(7), Some(8), None,
         Some(42), Some(42), None, Some(7), Some(8), None]);
    assert_eq!(fixture.set_optional_control(Some(&FailingControl)), Err("query failed"));
    assert_eq!(fixture.raw.calls.borrow().len(), 12);

    fixture.write_strings("A😀", Some("é😀"))?;
    fixture.write_strings(String::from(""), Some(""))?;
    fixture.write_strings("required", None)?;
    assert_eq!(*fixture.raw.strings.borrow(), [
        (vec![65, 0xd83d, 0xde00, 0], Some(vec![0xe9, 0xd83d, 0xde00, 0])),
        (vec![0], Some(vec![0])),
        ("required".encode_utf16().chain(Some(0)).collect(), None),
    ]);
    Ok(())
}
