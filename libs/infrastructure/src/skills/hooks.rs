/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use async_trait::async_trait;

/// The verdict returned by a ToolHook.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HookVerdict {
    /// Allow the tool execution to proceed.
    Allow,
    /// Deny the tool execution and abort.
    Deny(String),
    /// ユーザーに実行可否を確認する（タイムアウト付き）
    Ask {
        reason: String,
        timeout_ms: u64,
        allow_default: bool,
    },
    /// Transform the input payload before proceeding.
    Transform(String),
}

/// A hook that can intercept tool usage before and after execution.
#[async_trait]
pub trait ToolHook: Send + Sync {
    /// Called before the tool is executed.
    async fn pre_exec(&self, tool_name: &str, input: &str) -> HookVerdict;

    /// Called after the tool is executed successfully.
    async fn post_exec(&self, tool_name: &str, output: &str) -> HookVerdict;
}

/// A chain of hooks executed in sequence.
#[derive(Default)]
pub struct HookChain {
    hooks: Vec<Box<dyn ToolHook>>,
}

impl HookChain {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn add_hook(&mut self, hook: Box<dyn ToolHook>) {
        self.hooks.push(hook);
    }

    pub async fn execute_pre(&self, tool_name: &str, initial_input: &str) -> HookVerdict {
        let mut current_input = std::borrow::Cow::Borrowed(initial_input);
        let mut transformed = false;

        for hook in &self.hooks {
            match hook.pre_exec(tool_name, &current_input).await {
                HookVerdict::Deny(reason) => return HookVerdict::Deny(reason),
                HookVerdict::Ask {
                    reason,
                    timeout_ms,
                    allow_default,
                } => {
                    return HookVerdict::Ask {
                        reason,
                        timeout_ms,
                        allow_default,
                    };
                }
                HookVerdict::Transform(new_input) => {
                    current_input = std::borrow::Cow::Owned(new_input);
                    transformed = true;
                }
                HookVerdict::Allow => {}
            }
        }

        if transformed {
            HookVerdict::Transform(current_input.into_owned())
        } else {
            HookVerdict::Allow
        }
    }

    pub async fn execute_post(&self, tool_name: &str, initial_output: &str) -> HookVerdict {
        let mut current_output = std::borrow::Cow::Borrowed(initial_output);
        let mut transformed = false;

        for hook in &self.hooks {
            match hook.post_exec(tool_name, &current_output).await {
                HookVerdict::Deny(reason) => return HookVerdict::Deny(reason),
                HookVerdict::Ask {
                    reason,
                    timeout_ms,
                    allow_default,
                } => {
                    return HookVerdict::Ask {
                        reason,
                        timeout_ms,
                        allow_default,
                    };
                }
                HookVerdict::Transform(new_output) => {
                    current_output = std::borrow::Cow::Owned(new_output);
                    transformed = true;
                }
                HookVerdict::Allow => {}
            }
        }

        if transformed {
            HookVerdict::Transform(current_output.into_owned())
        } else {
            HookVerdict::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct TestHook {
        pre_result: HookVerdict,
        post_result: HookVerdict,
        call_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ToolHook for TestHook {
        async fn pre_exec(&self, _tool_name: &str, _input: &str) -> HookVerdict {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.pre_result.clone()
        }

        async fn post_exec(&self, _tool_name: &str, _output: &str) -> HookVerdict {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.post_result.clone()
        }
    }

    #[tokio::test]
    async fn test_hook_chain_allow() {
        let count = Arc::new(AtomicUsize::new(0));
        let hook = TestHook {
            pre_result: HookVerdict::Allow,
            post_result: HookVerdict::Allow,
            call_count: count.clone(),
        };

        let mut chain = HookChain::new();
        chain.add_hook(Box::new(hook));

        let verdict = chain.execute_pre("test_tool", "input_data").await;
        assert_eq!(verdict, HookVerdict::Allow);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_hook_chain_deny_aborts() {
        let count1 = Arc::new(AtomicUsize::new(0));
        let hook1 = TestHook {
            pre_result: HookVerdict::Deny("Blocked by policy".to_string()),
            post_result: HookVerdict::Allow,
            call_count: count1.clone(),
        };

        let count2 = Arc::new(AtomicUsize::new(0));
        let hook2 = TestHook {
            pre_result: HookVerdict::Allow,
            post_result: HookVerdict::Allow,
            call_count: count2.clone(),
        };

        let mut chain = HookChain::new();
        chain.add_hook(Box::new(hook1));
        chain.add_hook(Box::new(hook2));

        let verdict = chain.execute_pre("test_tool", "input_data").await;
        assert_eq!(verdict, HookVerdict::Deny("Blocked by policy".to_string()));

        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_hook_chain_transform() {
        let mut chain = HookChain::new();

        struct TransformHook;
        #[async_trait]
        impl ToolHook for TransformHook {
            async fn pre_exec(&self, _tool_name: &str, input: &str) -> HookVerdict {
                HookVerdict::Transform(format!("{} + transformed", input))
            }
            async fn post_exec(&self, _tool_name: &str, _output: &str) -> HookVerdict {
                HookVerdict::Allow
            }
        }

        struct CheckingHook;
        #[async_trait]
        impl ToolHook for CheckingHook {
            async fn pre_exec(&self, _tool_name: &str, input: &str) -> HookVerdict {
                if input == "base + transformed" {
                    HookVerdict::Allow
                } else {
                    HookVerdict::Deny("Wrong input".to_string())
                }
            }
            async fn post_exec(&self, _tool_name: &str, _output: &str) -> HookVerdict {
                HookVerdict::Allow
            }
        }

        chain.add_hook(Box::new(TransformHook));
        chain.add_hook(Box::new(CheckingHook));

        let verdict = chain.execute_pre("test_tool", "base").await;
        assert_eq!(
            verdict,
            HookVerdict::Transform("base + transformed".to_string())
        );
    }

    #[tokio::test]
    async fn test_hook_chain_ask_aborts() {
        let count1 = Arc::new(AtomicUsize::new(0));
        let hook1 = TestHook {
            pre_result: HookVerdict::Ask {
                reason: "Needs approval".to_string(),
                timeout_ms: 5000,
                allow_default: false,
            },
            post_result: HookVerdict::Allow,
            call_count: count1.clone(),
        };

        let count2 = Arc::new(AtomicUsize::new(0));
        let hook2 = TestHook {
            pre_result: HookVerdict::Allow,
            post_result: HookVerdict::Allow,
            call_count: count2.clone(),
        };

        let mut chain = HookChain::new();
        chain.add_hook(Box::new(hook1));
        chain.add_hook(Box::new(hook2));

        let verdict = chain.execute_pre("test_tool", "input_data").await;
        assert_eq!(
            verdict,
            HookVerdict::Ask {
                reason: "Needs approval".to_string(),
                timeout_ms: 5000,
                allow_default: false,
            }
        );

        // Ask が返されたら後続のフックは実行されない
        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 0);
    }
}
