//! Handwritten, model-independent registration for Rust-authored value
//! converters. Application-owned generated code supplies the
//! per-schema `ValueConverters` trait and a dispatch bridge; this module owns
//! the COM object lifetime, application-scoped registration, and conflict
//! rejection required by the architecture review.

use crate::{AppScope, Error, Result};
use avalonia_sys as sys;
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
