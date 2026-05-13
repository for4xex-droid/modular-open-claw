import os

files = [
    "libs/infrastructure/src/soul_mutator.rs",
    "libs/infrastructure/src/artifact_store_tests.rs",
    "libs/infrastructure/src/immune_system.rs",
    "libs/infrastructure/src/dream_state.rs",
    "libs/infrastructure/src/test_utils.rs"
]

method = """
    async fn append_job_karma_directives(&self, _job_id: &str, _hint: &str) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
"""

for file_path in files:
    with open(file_path, "r") as f:
        content = f.read()
    
    # TaskRegistry trait block implementation has cancel_job, we can insert our method after it
    if "async fn cancel_job" in content:
        content = content.replace(
            "async fn cancel_job(&self, _job_id: &str) -> Result<(), AiomeError> { Ok(()) }",
            "async fn cancel_job(&self, _job_id: &str) -> Result<(), AiomeError> { Ok(()) }\n" + method.replace("aiome_core::error::AiomeError", "AiomeError")
        )
        content = content.replace(
            "async fn cancel_job(&self, _job_id: &str) -> Result<(), aiome_core::error::AiomeError> { Ok(()) }",
            "async fn cancel_job(&self, _job_id: &str) -> Result<(), aiome_core::error::AiomeError> { Ok(()) }\n" + method
        )
        
    with open(file_path, "w") as f:
        f.write(content)
