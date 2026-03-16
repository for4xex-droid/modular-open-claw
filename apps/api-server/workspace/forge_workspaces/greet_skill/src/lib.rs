/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub fn greet(name: &str) -> String {
    format!("Hello, {} from Aiome Forge!", name)
}