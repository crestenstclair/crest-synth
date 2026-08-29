/// Maximum number of created Patches the prepared real-time graph can own.
///
/// This is a structural audio capacity, not a navigation limit: interaction
/// may still represent the trailing empty Patch position when all slots are
/// occupied.
pub const MAX_ACTIVE_PATCHES: usize = 16;

#[cfg(test)]
mod tests {
    use super::MAX_ACTIVE_PATCHES;

    #[test]
    fn prepared_graph_capacity_remains_sixteen() {
        assert_eq!(MAX_ACTIVE_PATCHES, 16);
    }
}
