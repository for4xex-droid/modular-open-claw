# ADR-042: Provider-Aware LLM Generation Cost Deduction Model

## Status
Accepted

## Context
Aiome has a decentralized agent economy integrated with the `Project-Nurture` economic ledger via the `CommerceEngine` trait.
While we have a basic `CostCircuitBreaker` to cap cumulative spending, we lack a provider-aware billing system that charges agents in real-time for their actual Large Language Model (LLM) usage. 

To achieve full financial sustainability, the cost of cloud LLM tokens (OpenAI, Gemini, Claude) must be calculated based on the specific model used and immediately deducted from the agent's balance in motivation coins.

## Decision
We will implement an automated, provider-aware billing integration for LLM generation costs in the `api-server`.

### 1. Model Pricing and Conversion Rate
We will leverage the existing pricing definitions in `libs/infrastructure/src/llm/dynamic.rs` (`model_pricing`). 

The exchange rate is established as:
* **`1 USD = 1,000 Coins`** (1 millidollar = 1 coin).

#### Target Prices (USD per 1 Million Tokens)
| Model | Input Rate ($) | Output Rate ($) | Input Coins / 1K Tokens | Output Coins / 1K Tokens |
|---|---|---|---|---|
| `gpt-4o` | 5.00 | 15.00 | 5.0 | 15.0 |
| `gpt-4.1` | 2.00 | 8.00 | 2.0 | 8.0 |
| `gpt-4.1-mini` | 0.40 | 1.60 | 0.4 | 1.6 |
| `claude-3-5-sonnet...` | 3.00 | 15.00 | 3.0 | 15.0 |
| `claude-opus-4...` | 15.00 | 75.00 | 15.0 | 75.0 |
| `gemini-1.5-pro...` | 1.25 | 5.00 | 1.25 | 5.0 |
| `gemini-2.5-flash` | 0.15 | 0.60 | 0.15 | 0.6 |
| `gemini-2.5-pro` | 1.25 | 10.00 | 1.25 | 10.0 |
| *Local Models* (Ollama, LM Studio) | 0.00 | 0.00 | 0.0 | 0.0 |

### 2. Micro-Billing Logic and Rounding
The token cost in USD is calculated as:
$$Cost_{USD} = \frac{Tokens_{In} \times Rate_{In} + Tokens_{Out} \times Rate_{Out}}{1,000,000}$$

The deduction amount in Coins is:
$$Amount_{Coins} = \lceil Cost_{USD} \times 1,000 \rceil$$

* Minimum charge is **1 Coin** if the cost is greater than 0.
* If the cost is exactly 0 (e.g., local model or no tokens consumed), the charge is 0.

### 3. Execution Pipeline and Non-Blocking Spawn
To prevent blocking client-facing streaming or critical agent execution loops with billing API network latency:
1. When a completion request (unary or streaming) completes successfully, the actual input and output tokens are extracted from `LlmResponse` metadata.
2. The system calculates the token cost.
3. If the cost is greater than 0, the `CommerceEngine::deduct_generation_cost` is invoked inside a non-blocking `tokio::spawn` task.

```
[LlmProvider Response]
       │ (with token counts)
       ▼
[Calculate Token Cost]
       │ (USD -> Coins, Ceil)
       ▼
[tokio::spawn Background task]
       │
       ▼
[CommerceEngine::deduct_generation_cost]
       │
       ▼
[POST /internal/deduct (Nurture Ledger)]
```

## Consequences
* **Real-time Cost Recovery**: Every generative prompt executed by the agent will be billed dynamically, ensuring motivationstudio, LLC does not run at a loss during cloud LLM operations.
* **Fail-Safe Robustness**: If the deduction API call fails due to a temporary network issue, the error will be logged as a warning, allowing the system to fail-open so that critical agent execution is not aborted mid-run.
* **TDD and Mock Validation**: A dedicated test suite will be added to ensure model billing calculates correctly and triggers ledger synchronization.
