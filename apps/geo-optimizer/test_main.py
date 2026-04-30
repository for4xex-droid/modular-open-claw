from unittest.mock import patch
from fastapi.testclient import TestClient
from main import app

client = TestClient(app)

def test_health_endpoint():
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok", "service": "geo-optimizer"}

def test_audit_rejects_shell_metacharacters_in_topic():
    # Arrange: semicolon in topic
    payload = {
        "topic": "test topic; touch /tmp/pwned",
        "content": "Valid content"
    }
    # Act
    response = client.post("/audit", json=payload)
    # Assert
    assert response.status_code == 400
    assert "Invalid characters" in response.json().get("detail", "")

def test_audit_rejects_shell_metacharacters_in_content():
    # Arrange: ampersand in content
    payload = {
        "topic": "Safe topic",
        "content": "Some content & rm -rf /"
    }
    # Act
    response = client.post("/audit", json=payload)
    # Assert
    assert response.status_code == 400
    assert "Invalid characters" in response.json().get("detail", "")

def test_audit_rejects_redirect_operators():
    # Arrange: redirect operators
    payload = {
        "topic": "topic > /etc/passwd",
        "content": "content"
    }
    # Act
    response = client.post("/audit", json=payload)
    # Assert
    assert response.status_code == 400

def test_audit_handles_file_not_found_error():
    # Arrange
    payload = {
        "topic": "Safe topic",
        "content": "Safe content"
    }
    # Act
    with patch("subprocess.run") as mock_run:
        mock_run.side_effect = FileNotFoundError("No such file or directory: 'geo'")
        response = client.post("/audit", json=payload)
    # Assert
    assert response.status_code == 503
    assert "geo optimizer CLI not found" in response.json().get("detail", "")

def test_audit_handles_timeout():
    # Arrange
    import subprocess
    payload = {
        "topic": "Safe topic",
        "content": "Safe content"
    }
    # Act
    with patch("subprocess.run") as mock_run:
        mock_run.side_effect = subprocess.TimeoutExpired(cmd="geo", timeout=15)
        response = client.post("/audit", json=payload)
    # Assert
    assert response.status_code == 504
    assert "timed out" in response.json().get("detail", "")

def test_audit_returns_simulated_score_on_cli_failure():
    # Arrange: CLI returns non-zero exit code
    import subprocess
    payload = {
        "topic": "Safe topic",
        "content": "A" * 600  # >500 chars → score should be 75
    }
    # Act
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = subprocess.CompletedProcess(
            args=["geo"], returncode=1, stdout="", stderr="command not supported"
        )
        response = client.post("/audit", json=payload)
    # Assert
    assert response.status_code == 200
    data = response.json()
    assert data["score"] == 75

