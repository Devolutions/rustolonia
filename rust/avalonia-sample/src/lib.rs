//! Sample-owned view-model API generated from `view-model.ir.json`.
//!
//! Generic transport, descriptors, sinks and mounting live in `avalonia`.
//! This crate is the application-owned generated surface for the flagship
//! sample, using the same `--external-rust` seam as external consumers.

pub use avalonia::{
    AppScope, CancellationToken, ClipboardData, ConversionDirection, Error, MapKey, RangeBatch,
    RangeRequest, RecentFileList, Result, ScalarKind, ScalarValue,
};

pub mod view_model {
    pub use avalonia::view_model::{
        BatchCompletion, DynamicViewModel, ViewModelBatch, ViewModelSink,
    };
}

pub mod value_converter {
    pub use avalonia::value_converter::ValueConverterDispatch;
}

#[rustfmt::skip]
mod generated_view_models;
pub use generated_view_models::*;
