#!/bin/bash
sed -i '' '1i\
#![allow(clippy::too_many_arguments)]\
#![allow(clippy::type_complexity)]\
#![allow(clippy::new_without_default)]\
#![allow(clippy::should_implement_trait)]\
#![allow(clippy::field_reassign_with_default)]\
#![allow(clippy::map_identity)]\
' libs/infrastructure/src/lib.rs

# Also add them to libs/soul/src/lib.rs for the test errors
sed -i '' '1i\
#![allow(clippy::unwrap_used)]\
#![allow(clippy::field_reassign_with_default)]\
' libs/soul/src/lib.rs
