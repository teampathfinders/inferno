//! A robust Minecraft Bedrock dedicated server implementation.
//!
//! The aim of this software is to be correct and performant.

#![doc(html_favicon_url = "https://github.com/teampathfinders/mirai/blob/master/resources/logo.png?raw=true")]
#![doc(html_logo_url = "https://github.com/teampathfinders/mirai/blob/master/resources/logo.png?raw=true")]
#![warn(
    missing_docs,
    clippy::expect_used,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::impl_trait_in_params,
    clippy::let_underscore_untyped,
    clippy::missing_assert_message,
    clippy::mutex_atomic,
    clippy::undocumented_unsafe_blocks,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::str_to_string,
    clippy::clone_on_ref_ptr,
    clippy::nursery,
    clippy::default_trait_access,
    clippy::doc_link_with_quotes,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::explicit_iter_loop,
    clippy::implicit_clone,
    clippy::index_refutable_slice,
    clippy::inefficient_to_string,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::large_stack_arrays,
    clippy::manual_instant_elapsed,
    clippy::manual_let_else,
    clippy::match_bool,
    clippy::missing_fields_in_debug,
    clippy::missing_panics_doc,
    clippy::redundant_closure_for_method_calls,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unused_self,
    clippy::unused_async
)]
#![allow(dead_code)]
#![allow(clippy::use_self)]

pub mod command;
pub mod config;
pub mod forms;
pub mod instance;
pub mod item;
pub mod level;
pub mod net;
pub mod inventory;

#[cfg(test)]
mod test;
