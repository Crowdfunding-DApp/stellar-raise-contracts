use soroban_sdk::String;

pub const MAX_STRING_LEN: u32 = 256;
pub const MAX_CONTRIBUTORS: u32 = 1_000;
pub const MAX_MILESTONES: u32 = 20;

pub fn validate_title(s: &String) -> bool {
    s.len() <= MAX_STRING_LEN
}
pub fn validate_description(s: &String) -> bool {
    s.len() <= MAX_STRING_LEN
}
pub fn validate_social_links(s: &String) -> bool {
    s.len() <= MAX_STRING_LEN
}
pub fn validate_roadmap_description(s: &String) -> bool {
    s.len() <= MAX_STRING_LEN
}
pub fn validate_bonus_goal_description(s: &String) -> bool {
    s.len() <= MAX_STRING_LEN
}
pub fn validate_metadata_total_length(len: u32) -> bool {
    len <= (MAX_STRING_LEN * 5)
}
pub fn validate_contributor_capacity(len: u32) -> bool {
    len < MAX_CONTRIBUTORS
}
pub fn validate_pledger_capacity(len: u32) -> bool {
    len < MAX_CONTRIBUTORS
}
pub fn validate_roadmap_capacity(len: u32) -> bool {
    len < 20
}
pub fn validate_stretch_goal_capacity(len: u32) -> bool {
    len < 10
}
pub fn validate_milestone_capacity(len: u32) -> bool {
    len < MAX_MILESTONES
}
pub fn validate_milestone_description(s: &String) -> bool {
    !s.is_empty() && s.len() <= MAX_STRING_LEN
}
