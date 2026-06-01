/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum BuzzTemplate {
    TechnicalInsight,
    MilestoneAnnouncement,
    CommunityQuestion,
    ControversialTake,
}

impl BuzzTemplate {
    pub fn target_signals(&self) -> Vec<String> {
        match self {
            BuzzTemplate::TechnicalInsight => {
                vec!["favorite".into(), "repost".into(), "dwell".into()]
            }
            BuzzTemplate::MilestoneAnnouncement => vec!["favorite".into(), "repost".into()],
            BuzzTemplate::CommunityQuestion => vec!["reply".into(), "quote".into(), "dwell".into()],
            BuzzTemplate::ControversialTake => vec!["reply".into(), "quote".into()],
        }
    }
}

pub fn get_system_prompt(template: &BuzzTemplate) -> String {
    match template {
        BuzzTemplate::TechnicalInsight => "You are an expert autonomous AI developer. Share a deep technical insight that will make other engineers pause and read (optimizing for dwell time and bookmarks/favorites). Be concise and use engaging formatting.".into(),
        BuzzTemplate::MilestoneAnnouncement => "You are an autonomous AI operating system celebrating a new milestone. Optimize for excitement and high engagement (reposts and favorites). Include emojis and a clear call to action.".into(),
        BuzzTemplate::CommunityQuestion => "Ask a thought-provoking question to the AI developer community. The goal is to maximize replies and quote-posts. Be somewhat provocative but professional.".into(),
        BuzzTemplate::ControversialTake => "Share a bold, slightly controversial take on the future of AI agents or software development. Maximize engagement via quotes and replies. Maintain a confident, cutting-edge persona.".into(),
    }
}

pub fn build_user_prompt(trend_source: &str, project_context: &str) -> String {
    format!(
        "Create a viral social media post for X (Twitter).\nContext: {}\nTrend Source Insight: {}\nEnsure the post is extremely engaging, fits within 280 characters, and includes relevant hashtags.",
        project_context, trend_source
    )
}
