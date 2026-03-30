#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    // ── ProfileSample ─────────────────────────────────────────────────────────

    #[test]
    fn test_profile_sample_new() {
        let s = ProfileSample::new("op", 1_000, 512);
        assert_eq!(s.label, "op");
        assert_eq!(s.instructions, 1_000);
        assert_eq!(s.memory_bytes, 512);
    }

    #[test]
    fn test_profile_sample_within_budget() {
        let s = ProfileSample::new("op", BUDGET_INSTRUCTION_LIMIT, 0);
        assert!(s.is_within_budget());
    }

    #[test]
    fn test_profile_sample_exceeds_budget() {
        let s = ProfileSample::new("op", BUDGET_INSTRUCTION_LIMIT + 1, 0);
        assert!(!s.is_within_budget());
    }

    #[test]
    fn test_profile_sample_zero_instructions_within_budget() {
        let s = ProfileSample::new("op", 0, 0);
        assert!(s.is_within_budget());
    }

    // ── ProfileReport::new ────────────────────────────────────────────────────

    #[test]
    fn test_profile_report_new_empty() {
        let r = ProfileReport::new();
        assert_eq!(r.sample_count(), 0);
        assert_eq!(r.total_instructions, 0);
        assert_eq!(r.peak_memory_bytes, 0);
    }

    // ── ProfileReport::add_sample ─────────────────────────────────────────────

    #[test]
    fn test_profile_report_add_sample() {
        let mut r = ProfileReport::new();
        r.add_sample(ProfileSample::new("a", 100, 50));
        assert_eq!(r.sample_count(), 1);
        assert_eq!(r.total_instructions, 100);
        assert_eq!(r.peak_memory_bytes, 50);
    }

    #[test]
    fn test_profile_report_max_samples_not_exceeded() {
        let mut r = ProfileReport::new();
        for _ in 0..=MAX_SAMPLES {
            r.add_sample(ProfileSample::new("x", 1, 1));
        }
        assert_eq!(r.sample_count(), MAX_SAMPLES);
    }

    #[test]
    fn test_profile_report_peak_memory_updated() {
        let mut r = ProfileReport::new();
        r.add_sample(ProfileSample::new("a", 0, 100));
        r.add_sample(ProfileSample::new("b", 0, 500));
        r.add_sample(ProfileSample::new("c", 0, 200));
        assert_eq!(r.peak_memory_bytes, 500);
    }

    // ── ProfileReport::average_instructions ──────────────────────────────────

    #[test]
    fn test_profile_report_average_instructions_empty() {
        let r = ProfileReport::new();
        assert_eq!(r.average_instructions(), 0);
    }

    #[test]
    fn test_profile_report_average_instructions() {
        let mut r = ProfileReport::new();
        r.add_sample(ProfileSample::new("a", 100, 0));
        r.add_sample(ProfileSample::new("b", 200, 0));
        assert_eq!(r.average_instructions(), 150);
    }

    // ── ProfileReport::budget_utilization_bps ────────────────────────────────

    #[test]
    fn test_budget_utilization_bps_zero() {
        let r = ProfileReport::new();
        assert_eq!(r.budget_utilization_bps(), 0);
    }

    #[test]
    fn test_budget_utilization_bps_full() {
        let mut r = ProfileReport::new();
        r.add_sample(ProfileSample::new("a", BUDGET_INSTRUCTION_LIMIT, 0));
        assert_eq!(r.budget_utilization_bps(), 10_000);
    }

    #[test]
    fn test_budget_utilization_bps_capped_at_10000() {
        let mut r = ProfileReport::new();
        // Add two samples each at the full limit → total exceeds limit
        r.add_sample(ProfileSample::new("a", BUDGET_INSTRUCTION_LIMIT, 0));
        r.add_sample(ProfileSample::new("b", BUDGET_INSTRUCTION_LIMIT, 0));
        assert_eq!(r.budget_utilization_bps(), 10_000);
    }

    #[test]
    fn test_budget_utilization_bps_half() {
        let mut r = ProfileReport::new();
        r.add_sample(ProfileSample::new("a", BUDGET_INSTRUCTION_LIMIT / 2, 0));
        assert_eq!(r.budget_utilization_bps(), 5_000);
    }

    // ── profile_operation ─────────────────────────────────────────────────────

    #[test]
    fn test_profile_operation() {
        let s = profile_operation("transfer", 42_000, 1_024);
        assert_eq!(s.label, "transfer");
        assert_eq!(s.instructions, 42_000);
        assert_eq!(s.memory_bytes, 1_024);
    }

    // ── check_budget ──────────────────────────────────────────────────────────

    #[test]
    fn test_check_budget_pass() {
        let s = ProfileSample::new("op", 1_000, 0);
        assert!(check_budget(&s));
    }

    #[test]
    fn test_check_budget_fail() {
        let s = ProfileSample::new("op", BUDGET_INSTRUCTION_LIMIT + 1, 0);
        assert!(!check_budget(&s));
    }

    // ── emit_profile_event ────────────────────────────────────────────────────

    #[test]
    fn test_emit_profile_event_does_not_panic() {
        let env = Env::default();
        emit_profile_event(&env, "test_op", 50_000);
    }
}
