/**
 * @name Inter-procedural taint tracking for Aiome + Nurture
 * @description Detects untrusted data flowing from HTTP handler parameters
 *              to filesystem operations or command execution sinks.
 *              Covers the full Aiome API server routes and Nurture infrastructure.
 * @kind path-problem
 * @problem.severity error
 * @security-severity 9.0
 * @precision high
 * @id aiome/taint-tracking
 * @tags security
 *       external/cwe/cwe-022
 *       external/cwe/cwe-078
 */

import rust
import codeql.rust.dataflow.DataFlow
import codeql.rust.dataflow.TaintTracking
import codeql.rust.Concepts

/**
 * Source: Any parameter in a function whose canonical path indicates
 * it is an HTTP handler (lives under a routes module), a webhook handler,
 * a WASM skill entry point, or is recognized by CodeQL's threat model.
 *
 * This broadly covers:
 *   - api_server::routes::*  (all Aiome API handlers)
 *   - *::webhook*            (Stripe, Polar webhooks)
 *   - *_handler              (convention-based handler functions)
 *   - WASM skill entry points
 *   - Any ActiveThreatModelSource from models-as-data
 */
class RemoteHandlerSource extends DataFlow::ParameterNode {
  RemoteHandlerSource() {
    this instanceof ActiveThreatModelSource
    or
    exists(Function f |
      (
        // All functions under routes:: modules (Aiome API server)
        f.getCanonicalPath().matches("%::routes::%") or
        // Webhook handlers
        f.getCanonicalPath().matches("%webhook%") or
        // Convention: functions ending with _handler
        f.getCanonicalPath().matches("%_handler") or
        // WASM skill entry points
        f.getCanonicalPath().matches("%wasm_skills%") or
        // Test functions (for adversarial test suite)
        f.getCanonicalPath().matches("%::test%")
      ) and
      this.asParameter() = f.getParamList().getAParam()
    )
  }
}

/**
 * Sink: dangerous operations that should never receive unsanitized user input.
 */
class DangerousSink extends DataFlow::Node {
  DangerousSink() {
    exists(Call c |
      c.getStaticTarget().getCanonicalPath() = [
        // Path injection sinks (CWE-022)
        "std::fs::write",
        "std::fs::read",
        "std::fs::read_to_string",
        "std::fs::remove_file",
        "std::fs::remove_dir",
        "std::fs::remove_dir_all",
        "std::fs::create_dir",
        "std::fs::create_dir_all",
        "std::fs::copy",
        "std::fs::rename",
        // Tokio async filesystem (CWE-022)
        "tokio::fs::write",
        "tokio::fs::read",
        "tokio::fs::read_to_string",
        "tokio::fs::remove_file",
        "tokio::fs::remove_dir_all",
        "tokio::fs::create_dir_all",
        // Command injection sinks (CWE-078)
        "<std::process::Command>::new",
        "<std::process::Command>::output",
        "<std::process::Command>::spawn",
        "<std::process::Command>::status",
        "<std::process::Command>::arg",
        "<std::process::Command>::args",
        // Tokio command (CWE-078)
        "<tokio::process::Command>::new",
        "<tokio::process::Command>::arg",
        "<tokio::process::Command>::spawn"
      ] and
      this.asExpr() = c.getAnArgument()
    )
  }
}

module AiomeTaintConfig implements DataFlow::ConfigSig {
  predicate isSource(DataFlow::Node source) {
    source instanceof RemoteHandlerSource
  }

  predicate isSink(DataFlow::Node sink) {
    sink instanceof DangerousSink
  }
}

module AiomeTaintFlow = TaintTracking::Global<AiomeTaintConfig>;

import AiomeTaintFlow::PathGraph

from AiomeTaintFlow::PathNode source, AiomeTaintFlow::PathNode sink
where AiomeTaintFlow::flowPath(source, sink)
select sink.getNode(), source, sink,
  "Tainted data from $@ reaches a dangerous operation.",
  source.getNode(), "remote handler parameter"
