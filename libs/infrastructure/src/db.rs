/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Infrastructure DB Bridge
//!
//! This module now re-exports from `libs/shared` to support modularization
//! while maintaining backward compatibility for the `infrastructure` crate.

pub use shared::db::{DatabasePool, DatabaseTransaction};

/// Re-exporting macros from shared crate
#[macro_export]
macro_rules! sql_exec {
    ($($arg:tt)*) => {
        ::shared::sql_exec!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_fetch_all {
    ($($arg:tt)*) => {
        ::shared::sql_fetch_all!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_fetch_one {
    ($($arg:tt)*) => {
        ::shared::sql_fetch_one!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_fetch_optional {
    ($($arg:tt)*) => {
        ::shared::sql_fetch_optional!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_tx_exec {
    ($($arg:tt)*) => {
        ::shared::sql_tx_exec!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_tx_fetch_all {
    ($($arg:tt)*) => {
        ::shared::sql_tx_fetch_all!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_tx_fetch_one {
    ($($arg:tt)*) => {
        ::shared::sql_tx_fetch_one!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_tx_fetch_optional {
    ($($arg:tt)*) => {
        ::shared::sql_tx_fetch_optional!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_fetch_raw {
    ($($arg:tt)*) => {
        ::shared::sql_fetch_raw!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_fetch_raw_optional {
    ($($arg:tt)*) => {
        ::shared::sql_fetch_raw_optional!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_fetch_raw_one {
    ($($arg:tt)*) => {
        ::shared::sql_fetch_raw_one!($($arg)*)
    };
}

#[macro_export]
macro_rules! sql_scalar {
    ($($arg:tt)*) => {
        ::shared::sql_scalar!($($arg)*)
    };
}
