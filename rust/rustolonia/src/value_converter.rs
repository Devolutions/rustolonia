//! Handwritten, model-independent registration for Rust-authored value
//! converters. Application-owned generated code supplies the
//! per-schema `ValueConverters` trait and a dispatch bridge; this module owns
//! the COM object lifetime, application-scoped registration, and conflict
//! rejection required by the architecture review.

use crate::view::{markers, set_unless_updating, IntoProp, PropBinding, TwoWayAdapter};
use crate::{AppScope, Effect, Error, PropertyPath, Result, Signal};
use rustolonia_sys as sys;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

pub use sys::{ConversionDirection, ScalarKind, ScalarValue};

/// Bridges a generated dispatch shim to the raw ABI. Implementations must be
/// pure (no `ViewModel` state access, no locking) because the resulting
/// provider is invoked without any provider-level lock and may be called
/// concurrently from any thread that evaluates a binding.
pub trait ValueConverterDispatch: Send + Sync + 'static {
    #[allow(clippy::too_many_arguments)]
    fn convert(
        &self,
        converter_id: i32,
        direction: ConversionDirection,
        value: ScalarValue,
        parameter: ScalarValue,
        target_kind: ScalarKind,
        culture: &str,
    ) -> Result<ScalarValue>;
}

/// An application-owned scalar model path with its declared source type.
///
/// The declared kind is also passed to `ConvertBack`, including when the
/// current value is null. A two-way binding requires a writable path.
#[derive(Clone, Debug)]
pub struct ScalarPath {
    path: PropertyPath<ScalarValue>,
    kind: ScalarKind,
}

impl ScalarPath {
    pub fn new(kind: ScalarKind, path: PropertyPath<ScalarValue>) -> Result<Self> {
        if !matches!(
            kind,
            ScalarKind::String | ScalarKind::Int64 | ScalarKind::Boolean | ScalarKind::Double
        ) {
            return Err(Error::Binding(format!(
                "{kind:?} is not a supported scalar source type"
            )));
        }
        Ok(Self { path, kind })
    }
}

/// Associates an AXAML converter reference with an existing Rust dispatch.
///
/// The culture signal is read on each conversion, so changing it re-evaluates
/// mounted one-way bindings. Supply the same reference that appears in the
/// generated bindings, for example `x:Static StringConverters.IsNotNullOrEmpty`.
#[derive(Clone)]
pub struct RegisteredValueConverter {
    reference: String,
    converter_id: i32,
    dispatch: Arc<dyn ValueConverterDispatch>,
    culture: Signal<String>,
}

impl fmt::Debug for RegisteredValueConverter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredValueConverter")
            .field("reference", &self.reference)
            .field("converter_id", &self.converter_id)
            .finish_non_exhaustive()
    }
}

impl RegisteredValueConverter {
    pub fn new(
        reference: impl Into<String>,
        converter_id: i32,
        dispatch: Arc<dyn ValueConverterDispatch>,
        culture: Signal<String>,
    ) -> Result<Self> {
        let reference = reference.into();
        if reference.trim().is_empty() {
            return Err(Error::Binding(
                "converter reference must not be empty".to_string(),
            ));
        }
        Ok(Self {
            reference,
            converter_id,
            dispatch,
            culture,
        })
    }

    fn convert(
        &self,
        reference: &str,
        direction: ConversionDirection,
        value: ScalarValue,
        parameter: ScalarValue,
        target_kind: ScalarKind,
        culture_override: Option<&str>,
    ) -> Result<ScalarValue> {
        if self.reference != reference {
            return Err(Error::Binding(format!(
                "expected converter '{reference}', got '{}'",
                self.reference
            )));
        }
        let culture = culture_override
            .map(str::to_string)
            .unwrap_or_else(|| self.culture.get());
        self.dispatch.convert(
            self.converter_id,
            direction,
            value,
            parameter,
            target_kind,
            &culture,
        )
    }
}

/// A projected scalar property supported by the AXAML converter bridge.
pub trait ConverterTarget: Sized + 'static {
    const KIND: ScalarKind;
    fn from_scalar(value: ScalarValue) -> Result<Self>;
    fn into_scalar(self) -> ScalarValue;
}

fn target_mismatch(expected: ScalarKind, value: &ScalarValue) -> Error {
    Error::Binding(format!(
        "converter returned {value:?}, expected {expected:?}"
    ))
}

macro_rules! scalar_target {
    ($ty:ty, $kind:ident, $variant:ident) => {
        impl ConverterTarget for $ty {
            const KIND: ScalarKind = ScalarKind::$kind;

            fn from_scalar(value: ScalarValue) -> Result<Self> {
                match value {
                    ScalarValue::$variant(value) => Ok(value),
                    value => Err(target_mismatch(Self::KIND, &value)),
                }
            }

            fn into_scalar(self) -> ScalarValue {
                ScalarValue::$variant(self)
            }
        }
    };
}

scalar_target!(String, String, String);
scalar_target!(bool, Boolean, Boolean);
scalar_target!(i64, Int64, Int64);
scalar_target!(f64, Double, Double);

impl ConverterTarget for i32 {
    const KIND: ScalarKind = ScalarKind::Int64;

    fn from_scalar(value: ScalarValue) -> Result<Self> {
        match value {
            ScalarValue::Int64(value) => i32::try_from(value)
                .map_err(|_| Error::Binding(format!("converter integer {value} exceeds i32"))),
            value => Err(target_mismatch(Self::KIND, &value)),
        }
    }

    fn into_scalar(self) -> ScalarValue {
        ScalarValue::Int64(self.into())
    }
}

impl ConverterTarget for Option<bool> {
    const KIND: ScalarKind = ScalarKind::Boolean;

    fn from_scalar(value: ScalarValue) -> Result<Self> {
        match value {
            ScalarValue::Null => Ok(None),
            ScalarValue::Boolean(value) => Ok(Some(value)),
            value => Err(target_mismatch(Self::KIND, &value)),
        }
    }

    fn into_scalar(self) -> ScalarValue {
        self.map(ScalarValue::Boolean).unwrap_or(ScalarValue::Null)
    }
}

fn check_source_kind(kind: ScalarKind, value: &ScalarValue) -> Result<()> {
    if matches!(
        (kind, value),
        (_, ScalarValue::Null)
            | (ScalarKind::String, ScalarValue::String(_))
            | (ScalarKind::Int64, ScalarValue::Int64(_))
            | (ScalarKind::Boolean, ScalarValue::Boolean(_))
            | (ScalarKind::Double, ScalarValue::Double(_))
    ) {
        Ok(())
    } else {
        Err(Error::Binding(format!(
            "scalar path declared {kind:?} but returned {value:?}"
        )))
    }
}

fn converted_target<T: ConverterTarget>(value: ScalarValue) -> Result<Option<T>> {
    match value {
        ScalarValue::DoNothing => Ok(None),
        ScalarValue::Unset => Err(Error::Binding(
            "converter returned Unset, but AXAML fallback values are not supported".to_string(),
        )),
        value => T::from_scalar(value).map(Some),
    }
}

/// A scalar model path transformed by a registered converter.
pub struct ConvertedBinding<T: ConverterTarget> {
    source: ScalarPath,
    converter: RegisteredValueConverter,
    reference: &'static str,
    parameter: ScalarValue,
    culture: Option<&'static str>,
    two_way: bool,
    target: PhantomData<fn() -> T>,
}

impl<T: ConverterTarget> ConvertedBinding<T> {
    pub fn new(
        source: ScalarPath,
        converter: RegisteredValueConverter,
        reference: &'static str,
        parameter: ScalarValue,
        culture: Option<&'static str>,
    ) -> Self {
        Self {
            source,
            converter,
            reference,
            parameter,
            culture,
            two_way: false,
            target: PhantomData,
        }
    }

    pub fn two_way(mut self) -> Self {
        self.two_way = true;
        self
    }
}

impl<T: ConverterTarget> IntoProp<T, markers::ConvertedMarker> for ConvertedBinding<T> {
    fn into_prop(self) -> PropBinding<T> {
        let ScalarPath { path, kind } = self.source;
        let reader = path.read;
        let writer = path.write;
        let converter = self.converter;
        let reference = self.reference;
        let parameter = self.parameter;
        let culture = self.culture;

        if !self.two_way {
            return PropBinding::FallibleOptional(Box::new(move || {
                let source = reader()?;
                check_source_kind(kind, &source)?;
                let converted = converter.convert(
                    reference,
                    ConversionDirection::Convert,
                    source,
                    parameter.clone(),
                    T::KIND,
                    culture,
                )?;
                converted_target(converted)
            }));
        }

        let is_writable = writer.is_some();
        let read_converter = converter.clone();
        let read_parameter = parameter.clone();
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                if !is_writable {
                    return Err(Error::ReadOnlyBindingPath);
                }
                Effect::new(move || {
                    let source = reader()?;
                    check_source_kind(kind, &source)?;
                    let converted = read_converter.convert(
                        reference,
                        ConversionDirection::Convert,
                        source,
                        read_parameter.clone(),
                        T::KIND,
                        culture,
                    )?;
                    if let Some(value) = converted_target(converted)? {
                        set_unless_updating(&updating, &*setter, value)?;
                    }
                    Ok(())
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: T| {
                let converted = converter.convert(
                    reference,
                    ConversionDirection::ConvertBack,
                    value.into_scalar(),
                    parameter.clone(),
                    kind,
                    culture,
                )?;
                match converted {
                    ScalarValue::DoNothing => Ok(()),
                    ScalarValue::Unset => Err(Error::Binding(
                        "converter returned Unset during ConvertBack".to_string(),
                    )),
                    value => {
                        check_source_kind(kind, &value)?;
                        writer.as_ref().ok_or(Error::ReadOnlyBindingPath)?(value)
                    }
                }
            }),
        })
    }
}

impl AppScope {
    /// Registers a generated value-converter dispatch as the single
    /// application-scoped provider. Rejects a conflicting registration
    /// (a different provider already active) rather than silently replacing
    /// it, and keeps the provider alive for the application's lifetime so it
    /// resolves for DataTemplate/ControlTemplate realization deferred past
    /// window construction.
    pub fn register_value_converter_dispatch(
        &self,
        dispatch: impl ValueConverterDispatch,
    ) -> Result<()> {
        let dispatch: Arc<dyn ValueConverterDispatch> = Arc::new(dispatch);
        let convert: Arc<sys::ConvertFn> = Arc::new(
            move |converter_id, direction, value, parameter, target_kind, culture| {
                dispatch
                    .convert(
                        converter_id,
                        direction,
                        value,
                        parameter,
                        target_kind,
                        culture,
                    )
                    .map_err(to_converter_abi_error)
            },
        );
        let provider = sys::rust_value_converter_provider(convert);
        self.application()
            .set_value_converter_provider(Some(&provider))?;
        self.retain_object(provider);
        Ok(())
    }
}

// Single-view hosts (browser, Android) keep the session for the process lifetime.
#[cfg_attr(any(target_os = "emscripten", target_os = "android"), allow(dead_code))]
pub(crate) fn clear_value_converter_provider(application: &sys::ComPtr<sys::IAvnApplication>) {
    // Best effort: releasing at shutdown is always valid, including when no
    // provider was ever registered.
    let _ = application.set_value_converter_provider(None);
}

fn to_converter_abi_error(error: Error) -> sys::ConverterAbiError {
    match error {
        Error::Abi(error) => error.into(),
        other => sys::ConverterAbiError::new(sys::E_FAIL, other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReactiveScope;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;

    #[derive(Default)]
    struct TestDispatch {
        calls: Mutex<Vec<(ConversionDirection, ScalarKind, ScalarValue, String)>>,
    }

    impl ValueConverterDispatch for TestDispatch {
        fn convert(
            &self,
            converter_id: i32,
            direction: ConversionDirection,
            value: ScalarValue,
            parameter: ScalarValue,
            target_kind: ScalarKind,
            culture: &str,
        ) -> Result<ScalarValue> {
            self.calls.lock().unwrap().push((
                direction,
                target_kind,
                parameter,
                culture.to_string(),
            ));
            match (converter_id, direction, value) {
                (1, ConversionDirection::Convert, ScalarValue::String(value)) => {
                    Ok(ScalarValue::Boolean(!value.is_empty()))
                }
                (2, ConversionDirection::Convert, ScalarValue::Int64(value)) => {
                    Ok(ScalarValue::String(value.to_string()))
                }
                (2, ConversionDirection::ConvertBack, ScalarValue::String(value))
                    if value == "keep" =>
                {
                    Ok(ScalarValue::DoNothing)
                }
                (2, ConversionDirection::ConvertBack, ScalarValue::String(value)) => value
                    .parse::<i64>()
                    .map(ScalarValue::Int64)
                    .map_err(|_| Error::Binding("not an integer".to_string())),
                _ => Err(Error::Binding("unsupported test conversion".to_string())),
            }
        }
    }

    #[test]
    fn converter_tracks_source_and_culture_until_disposed() {
        let scope = ReactiveScope::new();
        let source = Signal::new(ScalarValue::String("hello".to_string()));
        let culture = Signal::new("en-US".to_string());
        let dispatch = Arc::new(TestDispatch::default());
        let converter = RegisteredValueConverter::new(
            "x:Static StringConverters.IsNotNullOrEmpty",
            1,
            dispatch.clone(),
            culture,
        )
        .unwrap();
        let path = ScalarPath::new(
            ScalarKind::String,
            PropertyPath::read_only(move || Ok(source.get())),
        )
        .unwrap();
        let seen = Rc::new(RefCell::new(Vec::new()));
        scope.run(|| {
            let binding = ConvertedBinding::<bool>::new(
                path,
                converter,
                "x:Static StringConverters.IsNotNullOrEmpty",
                ScalarValue::String("parameter".to_string()),
                None,
            );
            let binding: PropBinding<bool> = binding.into_prop();
            let PropBinding::FallibleOptional(mut compute) = binding else {
                panic!("expected converted binding");
            };
            let seen = seen.clone();
            Effect::new(move || {
                if let Some(value) = compute()? {
                    seen.borrow_mut().push(value);
                }
                Ok(())
            })
            .unwrap();
        });
        culture.set("fr-FR".to_string());
        source.set(ScalarValue::String(String::new()));
        assert_eq!(*seen.borrow(), [true, true, false]);
        let calls = dispatch.calls.lock().unwrap();
        assert_eq!(calls[0].1, ScalarKind::Boolean);
        assert_eq!(calls[0].2, ScalarValue::String("parameter".to_string()));
        assert_eq!(calls[1].3, "fr-FR");
        drop(calls);
        scope.dispose();
        source.set(ScalarValue::String("again".to_string()));
        assert_eq!(*seen.borrow(), [true, true, false]);
    }

    #[test]
    fn two_way_converter_passes_declared_source_kind_and_preserves_errors() {
        let scope = ReactiveScope::new();
        let source = Signal::new(7_i64);
        let culture = Signal::new("en-US".to_string());
        let dispatch = Arc::new(TestDispatch::default());
        let converter =
            RegisteredValueConverter::new("StaticResource Number", 2, dispatch.clone(), culture)
                .unwrap();
        let path = ScalarPath::new(
            ScalarKind::Int64,
            PropertyPath::writable(
                move || Ok(ScalarValue::Int64(source.get())),
                move |value| {
                    let ScalarValue::Int64(value) = value else {
                        return Err(Error::Binding("expected integer".to_string()));
                    };
                    source.set_if_changed(value);
                    Ok(())
                },
            ),
        )
        .unwrap();
        let seen = Rc::new(RefCell::new(Vec::new()));
        scope.run(|| {
            let binding = ConvertedBinding::<String>::new(
                path,
                converter,
                "StaticResource Number",
                ScalarValue::Null,
                Some("fr-FR"),
            )
            .two_way();
            let PropBinding::TwoWay(adapter) = binding.into_prop() else {
                panic!("expected two-way binding");
            };
            let seen = seen.clone();
            (adapter.bind_to_control)(
                Box::new(move |value| {
                    seen.borrow_mut().push(value);
                    Ok(())
                }),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
            (adapter.set_from_control)("42".to_string()).unwrap();
            (adapter.set_from_control)("keep".to_string()).unwrap();
            assert!((adapter.set_from_control)("invalid".to_string()).is_err());
        });
        assert_eq!(source.get(), 42);
        assert_eq!(*seen.borrow(), ["7", "42"]);
        let calls = dispatch.calls.lock().unwrap();
        assert!(calls.iter().any(|(direction, target, _, culture)| {
            *direction == ConversionDirection::ConvertBack
                && *target == ScalarKind::Int64
                && culture == "fr-FR"
        }));
        drop(calls);
        scope.dispose();
    }

    #[test]
    fn converter_rejects_bad_types_unset_and_wrong_registration() {
        assert!(converted_target::<bool>(ScalarValue::DoNothing)
            .unwrap()
            .is_none());
        assert!(converted_target::<bool>(ScalarValue::Unset).is_err());
        assert!(converted_target::<bool>(ScalarValue::Int64(1)).is_err());
        assert!(check_source_kind(ScalarKind::String, &ScalarValue::Int64(1)).is_err());
        assert!(ScalarPath::new(
            ScalarKind::Any,
            PropertyPath::read_only(|| Ok(ScalarValue::Null))
        )
        .is_err());

        let converter = RegisteredValueConverter::new(
            "StaticResource Correct",
            1,
            Arc::new(TestDispatch::default()),
            Signal::new("en-US".to_string()),
        )
        .unwrap();
        let path = ScalarPath::new(
            ScalarKind::String,
            PropertyPath::read_only(|| Ok(ScalarValue::String("value".to_string()))),
        )
        .unwrap();
        let binding: PropBinding<bool> = ConvertedBinding::<bool>::new(
            path,
            converter,
            "StaticResource Wrong",
            ScalarValue::Null,
            None,
        )
        .into_prop();
        let PropBinding::FallibleOptional(mut read) = binding else {
            panic!("expected converted binding");
        };
        assert!(read()
            .unwrap_err()
            .to_string()
            .contains("expected converter"));
    }
}
