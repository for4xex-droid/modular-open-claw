/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use serde::{Deserialize, Serialize};

/// Agent Card - ネットワーク上のエージェントの能力とエンドポイントを宣言する自己紹介状
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCard {
    pub name: String,
    pub version: String,
    pub skills: Vec<String>,
    pub capabilities: Vec<String>,
    pub endpoints: Endpoints,
    pub security: SecurityProfile,
    pub sla: SlaConfig,
    pub pricing: PricingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Endpoints {
    pub grpc: Option<String>,
    pub rest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityProfile {
    pub auth: Vec<String>,
    pub ztas: ZtasProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZtasProfile {
    pub did: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlaConfig {
    pub max_latency_ms: u64,
    pub availability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PricingConfig {
    pub protocol: String,
    pub base_rate: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_agent_card_serialization() {
        let card = AgentCard {
            name: "Aiome Shadow Worker".to_string(),
            version: "1.0".to_string(),
            skills: vec!["code_generation".to_string(), "devops_triage".to_string()],
            capabilities: vec!["gig/publish".to_string(), "profile/info".to_string()],
            endpoints: Endpoints {
                grpc: Some("grpc://node.aiome.local:50051".to_string()),
                rest: Some("https://node.aiome.local:8080".to_string()),
            },
            security: SecurityProfile {
                auth: vec!["oauth2".to_string(), "mtls".to_string()],
                ztas: ZtasProfile {
                    did: "did:key:z6MkhaXgBZDvotDkL5257faiztiuC2ZXpu258wtVGnQkERfN".to_string(),
                },
            },
            sla: SlaConfig {
                max_latency_ms: 200,
                availability: 0.999,
            },
            pricing: PricingConfig {
                protocol: "x402".to_string(),
                base_rate: "0.001 USDC/request".to_string(),
            },
        };

        let json_value = serde_json::to_value(&card).unwrap();

        let expected = json!({
            "name": "Aiome Shadow Worker",
            "version": "1.0",
            "skills": ["code_generation", "devops_triage"],
            "capabilities": ["gig/publish", "profile/info"],
            "endpoints": {
                "grpc": "grpc://node.aiome.local:50051",
                "rest": "https://node.aiome.local:8080"
            },
            "security": {
                "auth": ["oauth2", "mtls"],
                "ztas": { "did": "did:key:z6MkhaXgBZDvotDkL5257faiztiuC2ZXpu258wtVGnQkERfN" }
            },
            "sla": {
                "max_latency_ms": 200,
                "availability": 0.999
            },
            "pricing": {
                "protocol": "x402",
                "base_rate": "0.001 USDC/request"
            }
        });

        assert_eq!(json_value, expected);

        // Deserialization test
        let deserialized: AgentCard = serde_json::from_value(expected).unwrap();
        assert_eq!(card, deserialized);
    }
}
