#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    // ── SidechainMessage::new ─────────────────────────────────────────────────

    #[test]
    fn test_sidechain_message_new_valid() {
        let msg = SidechainMessage::new(1, 42, 128);
        assert!(msg.is_some());
        let m = msg.unwrap();
        assert_eq!(m.chain_id, 1);
        assert_eq!(m.sequence, 42);
        assert_eq!(m.payload_len, 128);
    }

    #[test]
    fn test_sidechain_message_new_invalid_chain_id() {
        let msg = SidechainMessage::new(MAX_CHAIN_ID + 1, 0, 0);
        assert!(msg.is_none());
    }

    #[test]
    fn test_sidechain_message_new_invalid_payload() {
        let msg = SidechainMessage::new(1, 0, MAX_PAYLOAD_BYTES + 1);
        assert!(msg.is_none());
    }

    #[test]
    fn test_sidechain_message_new_boundary_valid() {
        assert!(SidechainMessage::new(MAX_CHAIN_ID, 0, MAX_PAYLOAD_BYTES).is_some());
    }

    // ── SidechainMessage::is_valid ────────────────────────────────────────────

    #[test]
    fn test_sidechain_message_is_valid_true() {
        let msg = SidechainMessage { chain_id: 1, sequence: 0, payload_len: 100 };
        assert!(msg.is_valid());
    }

    #[test]
    fn test_sidechain_message_is_valid_false_chain_id() {
        let msg = SidechainMessage { chain_id: MAX_CHAIN_ID + 1, sequence: 0, payload_len: 0 };
        assert!(!msg.is_valid());
    }

    #[test]
    fn test_sidechain_message_is_valid_false_payload() {
        let msg = SidechainMessage { chain_id: 1, sequence: 0, payload_len: MAX_PAYLOAD_BYTES + 1 };
        assert!(!msg.is_valid());
    }

    // ── verify_message ────────────────────────────────────────────────────────

    #[test]
    fn test_verify_message_valid() {
        let msg = SidechainMessage::new(10, 1, 64).unwrap();
        assert!(verify_message(&msg));
    }

    #[test]
    fn test_verify_message_invalid() {
        let msg = SidechainMessage { chain_id: MAX_CHAIN_ID + 1, sequence: 0, payload_len: 0 };
        assert!(!verify_message(&msg));
    }

    // ── estimate_gas_savings ──────────────────────────────────────────────────

    #[test]
    fn test_estimate_gas_savings_zero_payload() {
        let expected = (MAX_PAYLOAD_BYTES as u32).saturating_mul(10);
        assert_eq!(estimate_gas_savings(0), expected);
    }

    #[test]
    fn test_estimate_gas_savings_max_payload() {
        assert_eq!(estimate_gas_savings(MAX_PAYLOAD_BYTES), 0);
    }

    #[test]
    fn test_estimate_gas_savings_half_payload() {
        let half = MAX_PAYLOAD_BYTES / 2; // 128
        let expected = (MAX_PAYLOAD_BYTES - half) as u32 * 10;
        assert_eq!(estimate_gas_savings(half), expected);
    }

    #[test]
    fn test_estimate_gas_savings_overflow_safe() {
        // payload_len > MAX_PAYLOAD_BYTES should saturate to 0, not underflow
        assert_eq!(estimate_gas_savings(MAX_PAYLOAD_BYTES + 1000), 0);
    }

    // ── RelayResult ───────────────────────────────────────────────────────────

    #[test]
    fn test_relay_result_gas_efficiency_pct() {
        let r = RelayResult::new(true, 5_000);
        assert_eq!(r.gas_efficiency_pct(), 50);
    }

    #[test]
    fn test_relay_result_gas_efficiency_pct_zero() {
        let r = RelayResult::new(false, 0);
        assert_eq!(r.gas_efficiency_pct(), 0);
    }

    #[test]
    fn test_relay_result_gas_efficiency_pct_full() {
        let r = RelayResult::new(true, 10_000);
        assert_eq!(r.gas_efficiency_pct(), 100);
    }

    // ── ChainStatus ───────────────────────────────────────────────────────────

    #[test]
    fn test_chain_status_active_is_active() {
        assert!(ChainStatus::Active.is_active());
    }

    #[test]
    fn test_chain_status_inactive_not_active() {
        assert!(!ChainStatus::Inactive.is_active());
    }

    #[test]
    fn test_chain_status_suspended_not_active() {
        assert!(!ChainStatus::Suspended.is_active());
    }

    // ── emit_relay_event ──────────────────────────────────────────────────────

    #[test]
    fn test_emit_relay_event_does_not_panic() {
        let env = Env::default();
        emit_relay_event(&env, 1, 42);
    }
}
