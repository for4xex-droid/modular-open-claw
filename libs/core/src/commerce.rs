/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

//! # 経済活動インターフェース (Commerce Interface)
//!
//! AIエージェントが自律的に経済活動（決済、購入、報酬受取）を行うためのインターフェースを定義する。
//! このモジュールは `nurture` feature が有効な場合のみ機能する。

pub use aiome_interface::commerce::{CommerceEngine, EconomicContext};
