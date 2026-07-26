use serde::{Deserialize, Serialize};

/// Stable semantic focus identity inside the PATCH context.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PatchControlId {
    #[serde(rename = "patch.engine")]
    Engine,
}

impl PatchControlId {
    pub const ALL: [Self; 1] = [Self::Engine];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Engine => "patch.engine",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PatchControlId;

    #[test]
    fn patch_control_id_is_one_stable_semantic_engine_identity() {
        assert_eq!(
            PatchControlId::surface_descriptor(),
            &[PatchControlId::Engine]
        );
        assert_eq!(PatchControlId::Engine.as_str(), "patch.engine");
        assert_eq!(
            serde_json::to_string(&PatchControlId::Engine).unwrap(),
            "\"patch.engine\""
        );
    }
}
