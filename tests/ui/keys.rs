//! Keys addressed to widgets through the public path: `Context::on_input`, the next
//! `Context::run`, and an external `Widget`. Ownership, ordering, lifecycle and priority.

mod address;
mod lifecycle;
mod order;
mod priority;
pub(crate) mod support;
