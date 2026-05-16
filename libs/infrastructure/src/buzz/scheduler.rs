use super::templates::BuzzTemplate;
use std::time::Duration;

pub struct BuzzScheduler {
    pub min_interval: Duration,
    pub max_daily_posts: u8,
    pub last_template: std::sync::RwLock<Option<BuzzTemplate>>,
}

impl BuzzScheduler {
    pub fn new(min_interval_mins: u64, max_daily_posts: u8) -> Self {
        Self {
            min_interval: Duration::from_secs(min_interval_mins * 60),
            max_daily_posts,
            last_template: std::sync::RwLock::new(None),
        }
    }

    pub fn can_publish(&self, last_published: std::time::SystemTime, daily_post_count: u8) -> bool {
        if daily_post_count >= self.max_daily_posts {
            return false;
        }

        // If elapsed() returns Err (clock rollback), we allow publishing
        // as a safe-side fallback — better to post than to silently block.
        if let Ok(elapsed) = last_published.elapsed() {
            if elapsed < self.min_interval {
                return false;
            }
        }

        true
    }

    pub fn next_template(&self) -> BuzzTemplate {
        let guard = match self.last_template.read() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        match &*guard {
            Some(BuzzTemplate::TechnicalInsight) => BuzzTemplate::CommunityQuestion,
            Some(BuzzTemplate::CommunityQuestion) => BuzzTemplate::MilestoneAnnouncement,
            Some(BuzzTemplate::MilestoneAnnouncement) => BuzzTemplate::ControversialTake,
            Some(BuzzTemplate::ControversialTake) | None => BuzzTemplate::TechnicalInsight,
        }
    }

    pub fn update_last_template(&self, template: BuzzTemplate) {
        match self.last_template.write() {
            Ok(mut guard) => *guard = Some(template),
            Err(poisoned) => *poisoned.into_inner() = Some(template),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_can_publish_within_interval() {
        let sched = BuzzScheduler::new(90, 5); // 90 min interval
        let last = SystemTime::now(); // just published
        assert!(
            !sched.can_publish(last, 0),
            "Should block if interval not elapsed"
        );
    }

    #[test]
    fn test_can_publish_after_interval() {
        let sched = BuzzScheduler::new(90, 5);
        let last = SystemTime::now() - Duration::from_secs(91 * 60); // 91 mins ago
        assert!(
            sched.can_publish(last, 0),
            "Should allow after interval elapsed"
        );
    }

    #[test]
    fn test_can_publish_daily_limit_reached() {
        let sched = BuzzScheduler::new(90, 3);
        let last = SystemTime::now() - Duration::from_secs(200 * 60); // long ago
        assert!(!sched.can_publish(last, 3), "Should block at daily limit");
        assert!(
            !sched.can_publish(last, 4),
            "Should block above daily limit"
        );
    }

    #[test]
    fn test_can_publish_zero_posts() {
        let sched = BuzzScheduler::new(0, 10); // no interval
        let last = SystemTime::now();
        assert!(
            sched.can_publish(last, 0),
            "Zero interval should always allow"
        );
    }

    #[test]
    fn test_next_template_rotation() {
        let sched = BuzzScheduler::new(90, 5);
        // None → TechnicalInsight
        assert_eq!(sched.next_template(), BuzzTemplate::TechnicalInsight);

        sched.update_last_template(BuzzTemplate::TechnicalInsight);
        assert_eq!(sched.next_template(), BuzzTemplate::CommunityQuestion);

        sched.update_last_template(BuzzTemplate::CommunityQuestion);
        assert_eq!(sched.next_template(), BuzzTemplate::MilestoneAnnouncement);

        sched.update_last_template(BuzzTemplate::MilestoneAnnouncement);
        assert_eq!(sched.next_template(), BuzzTemplate::ControversialTake);

        sched.update_last_template(BuzzTemplate::ControversialTake);
        assert_eq!(sched.next_template(), BuzzTemplate::TechnicalInsight);
    }
}
