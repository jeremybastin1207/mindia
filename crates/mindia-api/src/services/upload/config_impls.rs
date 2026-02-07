//! MediaUploadConfig implementations using MediaConfig / MediaLimits

use crate::state::MediaLimits;

use super::traits::MediaUploadConfig;

/// Adapter to use MediaLimits as MediaUploadConfig (e.g. from media.limits_for(MediaType)).
pub struct MediaLimitsConfig<'a> {
    pub limits: &'a MediaLimits,
    pub media_type_name: &'static str,
}

impl MediaUploadConfig for MediaLimitsConfig<'_> {
    fn max_file_size(&self) -> usize {
        self.limits.max_file_size
    }

    fn allowed_extensions(&self) -> &[String] {
        &self.limits.allowed_extensions
    }

    fn allowed_content_types(&self) -> &[String] {
        &self.limits.allowed_content_types
    }

    fn media_type_name(&self) -> &'static str {
        self.media_type_name
    }
}
