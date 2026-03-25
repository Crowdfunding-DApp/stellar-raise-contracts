#[cfg(test)]
mod tests {
    use super::campaign_goal_minimum::*;
    use soroban_sdk::{testutils::{Address as _, Events}, Address, Env, IntoVal, Symbol};

    #[test]
    fn test_valid_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = 500u64;

        create_campaign(env.clone(), creator.clone(), goal);

        // Verify event emission
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let event = events.get(0).unwrap();
        assert_eq!(event.0, env.current_contract_address());
        assert_eq!(event.1, (Symbol::new(&env, "campaign"), Symbol::new(&env, "created")).into_val(&env));
        let data = event.2;
        assert_eq!(data, (creator, goal).into_val(&env));
    }

    #[test]
    #[should_panic(expected = "Minimum campaign goal not met")]
    fn test_below_minimum_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = 50u64;

        create_campaign(env.clone(), creator.clone(), goal);
    }

    #[test]
    #[should_panic(expected = "Campaign goal must be non-zero")]
    fn test_zero_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = 0u64;

        create_campaign(env.clone(), creator.clone(), goal);
    }

    #[test]
    fn test_exact_minimum() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = MIN_CAMPAIGN_GOAL;

        create_campaign(env.clone(), creator.clone(), goal);
    }

    #[test]
    fn test_large_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = u64::MAX;

        create_campaign(env.clone(), creator.clone(), goal);
    }
}
