use crate::control::{EngineSelectionFailure, EngineSelectionRequestId};
use crate::kernel::PatchId;
use crate::synth::{
    ParameterId, SampleAssetError, SampleAssetId, SampleBrowserRow, SampleBrowserRowKind,
    SampleCatalogListing, SampleFolderId,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleAssetLifecycle {
    Loading,
    Validating,
    Preparing,
    Activating,
    Ready,
    Unavailable,
    Invalid,
    Cancelled,
    Failed(EngineSelectionFailure),
}

impl SampleAssetLifecycle {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Loading => "LOADING",
            Self::Validating => "VALIDATING",
            Self::Preparing => "PREPARING",
            Self::Activating => "ACTIVATING",
            Self::Ready => "READY",
            Self::Unavailable => "UNAVAILABLE",
            Self::Invalid => "INVALID",
            Self::Cancelled => "CANCELLED — UNCHANGED",
            Self::Failed(_) => "FAILED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SamplePreviewState {
    Idle,
    Held {
        asset_id: SampleAssetId,
    },
    Preparing {
        asset_id: SampleAssetId,
        /// Whether Start is still physically held when preparation completes.
        held: bool,
    },
    Playing {
        asset_id: SampleAssetId,
    },
    Stopping {
        asset_id: SampleAssetId,
    },
    Failed {
        asset_id: SampleAssetId,
        cause: SampleAssetError,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleBrowserState {
    catalog: BTreeMap<SampleFolderId, Result<SampleCatalogListing, SampleAssetError>>,
    patch_id: Option<PatchId>,
    asset_parameter_id: Option<ParameterId>,
    folder: SampleFolderId,
    rows: Vec<SampleBrowserRow>,
    lifecycle: SampleAssetLifecycle,
    requested_asset: Option<SampleAssetId>,
    request_id: Option<EngineSelectionRequestId>,
    preview_request_id: Option<EngineSelectionRequestId>,
    preview: SamplePreviewState,
}

impl Default for SampleBrowserState {
    fn default() -> Self {
        Self {
            catalog: BTreeMap::new(),
            patch_id: None,
            asset_parameter_id: None,
            folder: SampleFolderId::default(),
            rows: fallback_rows(&SampleFolderId::default()),
            lifecycle: SampleAssetLifecycle::Unavailable,
            requested_asset: None,
            request_id: None,
            preview_request_id: None,
            preview: SamplePreviewState::Idle,
        }
    }
}

impl SampleBrowserState {
    pub fn with_catalog(
        mut self,
        listings: impl IntoIterator<
            Item = (
                SampleFolderId,
                Result<SampleCatalogListing, SampleAssetError>,
            ),
        >,
    ) -> Self {
        self.catalog = listings.into_iter().collect();
        self
    }

    pub const fn patch_id(&self) -> Option<PatchId> {
        self.patch_id
    }

    pub const fn asset_parameter_id(&self) -> Option<&ParameterId> {
        self.asset_parameter_id.as_ref()
    }

    pub const fn folder(&self) -> &SampleFolderId {
        &self.folder
    }

    pub fn rows(&self) -> &[SampleBrowserRow] {
        &self.rows
    }

    pub const fn lifecycle(&self) -> SampleAssetLifecycle {
        self.lifecycle
    }

    pub const fn requested_asset(&self) -> Option<&SampleAssetId> {
        self.requested_asset.as_ref()
    }

    pub const fn request_id(&self) -> Option<EngineSelectionRequestId> {
        self.request_id
    }

    pub const fn preview(&self) -> &SamplePreviewState {
        &self.preview
    }

    pub const fn preview_request_id(&self) -> Option<EngineSelectionRequestId> {
        self.preview_request_id
    }

    pub const fn preview_is_held(&self) -> bool {
        matches!(
            self.preview,
            SamplePreviewState::Held { .. }
                | SamplePreviewState::Preparing { held: true, .. }
                | SamplePreviewState::Playing { .. }
        )
    }

    pub fn row(&self, id: &str) -> Option<&SampleBrowserRow> {
        self.rows.iter().find(|row| row.id() == id)
    }

    pub(crate) fn begin(&mut self, patch_id: PatchId, asset_parameter_id: ParameterId) {
        self.patch_id = Some(patch_id);
        self.asset_parameter_id = Some(asset_parameter_id);
        self.requested_asset = None;
        self.request_id = None;
        self.preview_request_id = None;
        self.preview = SamplePreviewState::Idle;
        self.load_folder(SampleFolderId::default());
    }

    pub(crate) fn load_folder(&mut self, folder: SampleFolderId) {
        self.preview = SamplePreviewState::Idle;
        self.preview_request_id = None;
        self.folder = folder.clone();
        match self.catalog.get(&folder) {
            Some(Ok(listing)) if listing.folder() == &folder => {
                self.rows = listing.rows().to_vec();
                self.lifecycle = SampleAssetLifecycle::Ready;
            }
            Some(Err(SampleAssetError::Unavailable)) | None => {
                self.rows = fallback_rows(&folder);
                self.lifecycle = SampleAssetLifecycle::Unavailable;
            }
            Some(Err(error)) => {
                self.rows = fallback_rows(&folder);
                self.lifecycle = if matches!(error, SampleAssetError::Cancelled) {
                    SampleAssetLifecycle::Cancelled
                } else {
                    SampleAssetLifecycle::Invalid
                };
            }
            Some(Ok(_)) => {
                self.rows = fallback_rows(&folder);
                self.lifecycle = SampleAssetLifecycle::Invalid;
            }
        }
    }

    /// Replaces one adapter-owned catalog result and reloads the visible
    /// folder when it is the one currently open. Focus repair remains an
    /// AppState responsibility because only the reducer owns FocusPath.
    pub(crate) fn refresh_listing(
        &mut self,
        folder: SampleFolderId,
        listing: Result<SampleCatalogListing, SampleAssetError>,
    ) -> bool {
        let reload = self.patch_id.is_some() && self.folder == folder;
        self.catalog.insert(folder.clone(), listing);
        if reload {
            self.load_folder(folder);
        }
        reload
    }

    pub(crate) fn preview_requested(
        &mut self,
        asset_id: SampleAssetId,
        request_id: EngineSelectionRequestId,
    ) {
        self.preview_request_id = Some(request_id);
        self.preview = SamplePreviewState::Preparing {
            asset_id,
            held: true,
        };
    }

    /// Releases the hold. Returns `true` only when a compatible active
    /// audition voice needs a fixed-size stop command.
    pub(crate) fn preview_stop(&mut self) -> bool {
        let command_required = matches!(self.preview, SamplePreviewState::Playing { .. });
        self.preview = match &self.preview {
            SamplePreviewState::Idle => SamplePreviewState::Idle,
            SamplePreviewState::Held { asset_id } => SamplePreviewState::Preparing {
                asset_id: asset_id.clone(),
                held: false,
            },
            SamplePreviewState::Preparing { asset_id, .. } => SamplePreviewState::Preparing {
                asset_id: asset_id.clone(),
                held: false,
            },
            SamplePreviewState::Playing { .. } | SamplePreviewState::Stopping { .. } => {
                SamplePreviewState::Idle
            }
            SamplePreviewState::Failed { .. } => SamplePreviewState::Idle,
        };
        if command_required {
            self.preview_request_id = None;
        }
        command_required
    }

    pub(crate) fn preview_activating(&mut self, request_id: EngineSelectionRequestId) {
        debug_assert_eq!(self.preview_request_id, Some(request_id));
    }

    /// Completes activation and reports whether Start is still held, which is
    /// the only point at which the reducer may emit `PreviewStart`.
    pub(crate) fn preview_ready(&mut self, request_id: EngineSelectionRequestId) -> bool {
        if self.preview_request_id != Some(request_id) {
            return false;
        }
        let next = match &self.preview {
            SamplePreviewState::Preparing {
                asset_id,
                held: true,
            } => Some(SamplePreviewState::Playing {
                asset_id: asset_id.clone(),
            }),
            SamplePreviewState::Preparing { held: false, .. } => {
                self.preview_request_id = None;
                self.preview = SamplePreviewState::Idle;
                return false;
            }
            _ => return false,
        };
        self.preview = next.expect("held preparation produces Playing");
        true
    }

    pub(crate) fn preview_failed(
        &mut self,
        request_id: EngineSelectionRequestId,
        failure: EngineSelectionFailure,
    ) {
        if self.preview_request_id != Some(request_id) {
            return;
        }
        let asset_id = match &self.preview {
            SamplePreviewState::Held { asset_id }
            | SamplePreviewState::Preparing { asset_id, .. }
            | SamplePreviewState::Playing { asset_id }
            | SamplePreviewState::Stopping { asset_id }
            | SamplePreviewState::Failed { asset_id, .. } => asset_id.clone(),
            SamplePreviewState::Idle => return,
        };
        let cause = match failure {
            EngineSelectionFailure::AssetUnavailable => SampleAssetError::Unavailable,
            EngineSelectionFailure::UnsupportedAssetFormat => SampleAssetError::UnsupportedEncoding,
            EngineSelectionFailure::AssetCapacityExceeded => {
                SampleAssetError::AssetScalarCapacityExceeded
            }
            EngineSelectionFailure::GraphCapacityExceeded => {
                SampleAssetError::GraphPcmCapacityExceeded
            }
            EngineSelectionFailure::Cancelled => SampleAssetError::Cancelled,
            EngineSelectionFailure::AllocationFailed => SampleAssetError::AllocationFailed,
            _ => SampleAssetError::MalformedPcm,
        };
        self.preview = SamplePreviewState::Failed { asset_id, cause };
        self.preview_request_id = None;
    }

    pub(crate) fn assignment_requested(
        &mut self,
        asset_id: SampleAssetId,
        request_id: EngineSelectionRequestId,
    ) {
        self.requested_asset = Some(asset_id);
        self.request_id = Some(request_id);
        self.preview_request_id = None;
        self.lifecycle = SampleAssetLifecycle::Loading;
        self.preview = SamplePreviewState::Idle;
    }

    pub(crate) fn assignment_lifecycle_advanced(
        &mut self,
        request_id: EngineSelectionRequestId,
        lifecycle: SampleAssetLifecycle,
    ) -> bool {
        if self.request_id != Some(request_id) {
            return false;
        }
        let valid = matches!(
            (self.lifecycle, lifecycle),
            (
                SampleAssetLifecycle::Loading,
                SampleAssetLifecycle::Validating
            ) | (
                SampleAssetLifecycle::Validating,
                SampleAssetLifecycle::Preparing
            )
        );
        if valid {
            self.lifecycle = lifecycle;
        }
        valid
    }

    pub(crate) fn assignment_activating(&mut self, request_id: EngineSelectionRequestId) {
        if self.request_id == Some(request_id) && self.lifecycle == SampleAssetLifecycle::Preparing
        {
            self.lifecycle = SampleAssetLifecycle::Activating;
        }
    }

    pub(crate) fn assignment_ready(&mut self, request_id: EngineSelectionRequestId) {
        if self.request_id == Some(request_id) {
            self.lifecycle = SampleAssetLifecycle::Ready;
            self.requested_asset = None;
            self.request_id = None;
        }
    }

    pub(crate) fn assignment_failed(
        &mut self,
        request_id: EngineSelectionRequestId,
        failure: EngineSelectionFailure,
    ) {
        if self.request_id != Some(request_id) {
            return;
        }
        self.lifecycle = match failure {
            EngineSelectionFailure::AssetUnavailable => SampleAssetLifecycle::Unavailable,
            EngineSelectionFailure::InvalidAsset
            | EngineSelectionFailure::UnsupportedAssetFormat
            | EngineSelectionFailure::AssetCapacityExceeded
            | EngineSelectionFailure::GraphCapacityExceeded => SampleAssetLifecycle::Invalid,
            EngineSelectionFailure::Cancelled => SampleAssetLifecycle::Cancelled,
            other => SampleAssetLifecycle::Failed(other),
        };
        self.requested_asset = None;
        self.request_id = None;
        self.preview_request_id = None;
        self.preview = SamplePreviewState::Idle;
    }

    pub(crate) fn cancelled(&mut self) {
        self.preview = SamplePreviewState::Idle;
        self.requested_asset = None;
        self.request_id = None;
        self.preview_request_id = None;
        self.lifecycle = SampleAssetLifecycle::Cancelled;
    }
}

fn fallback_rows(folder: &SampleFolderId) -> Vec<SampleBrowserRow> {
    vec![SampleBrowserRow::new(
        format!("cancel:{}", folder.as_str()),
        "CANCEL — UNCHANGED",
        SampleBrowserRowKind::Cancel,
        None,
    )
    .expect("static fallback browser row is valid")]
}
