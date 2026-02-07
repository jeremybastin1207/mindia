//! Plugin implementations – OpenAI (others from mindia-plugins)

#[cfg(feature = "plugin-openai-image-description")]
mod openai_image_description;

#[cfg(feature = "plugin-openai-image-description")]
pub use openai_image_description::OpenAiImageDescriptionPlugin;
