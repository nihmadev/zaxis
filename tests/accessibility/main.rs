//! The accessibility tree, its updates and the requests it accepts, without a window, a GPU
//! or a screen reader. Every update is also fed to `accesskit_consumer`, the model platform
//! adapters are built on: an update it rejects would panic a real adapter.
#![cfg(feature = "accesskit")]
#![allow(unused_imports, dropping_copy_types)]
#![allow(clippy::drop_non_drop, clippy::collapsible_match)]

mod basic;
mod collections;
mod controls;
mod custom;
mod overlays;
mod static_text;
mod support;
mod text_edit;
