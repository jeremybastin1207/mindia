//! Processing pipeline for chaining media operations

use crate::traits::{MediaProcessor, MediaTransformer};
use anyhow::Result;
use bytes::Bytes;
use std::sync::Arc;

/// Processing step in a pipeline
pub enum PipelineStep<P, T>
where
    T: MediaTransformer,
    T::Options: Clone,
{
    Processor(Arc<P>),
    Transformer(Arc<T>, T::Options),
}

/// Media processing pipeline
pub struct ProcessingPipeline<P, T>
where
    P: MediaProcessor,
    T: MediaTransformer,
    T::Options: Clone,
{
    steps: Vec<PipelineStep<P, T>>,
}

impl<P, T> ProcessingPipeline<P, T>
where
    P: MediaProcessor + Send + Sync,
    T: MediaTransformer + Send + Sync,
    T::Options: Clone,
{
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add a processing step (metadata extraction)
    pub fn add_processor(&mut self, processor: Arc<P>) {
        self.steps.push(PipelineStep::Processor(processor));
    }

    /// Add a transformation step
    pub fn add_transformer(&mut self, transformer: Arc<T>, options: T::Options) {
        self.steps
            .push(PipelineStep::Transformer(transformer, options));
    }

    /// Execute the pipeline on input data
    pub async fn execute(&self, mut data: Bytes) -> Result<(Bytes, Option<P::Metadata>)> {
        let mut metadata: Option<P::Metadata> = None;

        for step in &self.steps {
            match step {
                PipelineStep::Processor(processor) => {
                    // Extract metadata without modifying data
                    let meta = processor.extract_metadata(&data).await?;
                    metadata = Some(meta);
                }
                PipelineStep::Transformer(transformer, options) => {
                    // Apply transformation (options must be cloned for each use)
                    data = transformer.transform(&data, options.clone()).await?;
                }
            }
        }

        Ok((data, metadata))
    }
}

impl<P, T> Default for ProcessingPipeline<P, T>
where
    P: MediaProcessor + Send + Sync,
    T: MediaTransformer + Send + Sync,
    T::Options: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{MediaProcessor, MediaTransformer, TransformType};
    use async_trait::async_trait;
    use bytes::Bytes;
    use std::sync::Arc;

    // Mock processor for testing
    struct MockProcessor {
        metadata_value: String,
    }

    #[async_trait]
    impl MediaProcessor for MockProcessor {
        type Metadata = String;

        async fn extract_metadata(&self, _data: &[u8]) -> Result<Self::Metadata, anyhow::Error> {
            Ok(self.metadata_value.clone())
        }

        fn validate(&self, data: &[u8]) -> Result<(), anyhow::Error> {
            if data.is_empty() {
                Err(anyhow::anyhow!("Empty data"))
            } else {
                Ok(())
            }
        }

        fn get_dimensions(&self, _data: &[u8]) -> Option<(u32, u32)> {
            Some((100, 100))
        }
    }

    // Mock transformer for testing
    struct MockTransformer {
        transform_prefix: String,
    }

    #[async_trait]
    impl MediaTransformer for MockTransformer {
        type Options = String;

        async fn transform(
            &self,
            data: &[u8],
            options: Self::Options,
        ) -> Result<Bytes, anyhow::Error> {
            let mut result = self.transform_prefix.clone().into_bytes();
            result.extend_from_slice(data);
            result.extend_from_slice(options.as_bytes());
            Ok(Bytes::from(result))
        }

        fn supported_transforms(&self) -> Vec<TransformType> {
            vec![TransformType::ImageResize]
        }
    }

    #[tokio::test]
    async fn test_pipeline_new() {
        let pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        assert_eq!(pipeline.steps.len(), 0);
    }

    #[tokio::test]
    async fn test_pipeline_default() {
        let pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::default();
        assert_eq!(pipeline.steps.len(), 0);
    }

    #[tokio::test]
    async fn test_pipeline_add_processor() {
        let mut pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        let processor = Arc::new(MockProcessor {
            metadata_value: "test_metadata".to_string(),
        });

        pipeline.add_processor(processor);
        assert_eq!(pipeline.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_add_transformer() {
        let mut pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        let transformer = Arc::new(MockTransformer {
            transform_prefix: "prefix_".to_string(),
        });

        pipeline.add_transformer(transformer, "options".to_string());
        assert_eq!(pipeline.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_execute_processor_only() {
        let mut pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        let processor = Arc::new(MockProcessor {
            metadata_value: "extracted_metadata".to_string(),
        });

        pipeline.add_processor(processor);

        let input = Bytes::from("test_data");
        let (output, metadata) = pipeline.execute(input.clone()).await.unwrap();

        // Data should be unchanged
        assert_eq!(output, input);
        // Metadata should be extracted
        assert_eq!(metadata, Some("extracted_metadata".to_string()));
    }

    #[tokio::test]
    async fn test_pipeline_execute_transformer_only() {
        let mut pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        let transformer = Arc::new(MockTransformer {
            transform_prefix: "prefix_".to_string(),
        });

        pipeline.add_transformer(transformer, "_options".to_string());

        let input = Bytes::from("data");
        let (output, metadata) = pipeline.execute(input).await.unwrap();

        // Data should be transformed
        assert_eq!(output, Bytes::from("prefix_data_options"));
        // No metadata extracted
        assert_eq!(metadata, None);
    }

    #[tokio::test]
    async fn test_pipeline_execute_processor_then_transformer() {
        let mut pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        let processor = Arc::new(MockProcessor {
            metadata_value: "metadata".to_string(),
        });
        let transformer = Arc::new(MockTransformer {
            transform_prefix: "transformed_".to_string(),
        });

        pipeline.add_processor(processor);
        pipeline.add_transformer(transformer, "_opts".to_string());

        let input = Bytes::from("input");
        let (output, metadata) = pipeline.execute(input).await.unwrap();

        // Data should be transformed
        assert_eq!(output, Bytes::from("transformed_input_opts"));
        // Metadata should be extracted
        assert_eq!(metadata, Some("metadata".to_string()));
    }

    #[tokio::test]
    async fn test_pipeline_execute_multiple_transformers() {
        let mut pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        let transformer1 = Arc::new(MockTransformer {
            transform_prefix: "t1_".to_string(),
        });
        let transformer2 = Arc::new(MockTransformer {
            transform_prefix: "t2_".to_string(),
        });

        pipeline.add_transformer(transformer1, "_o1".to_string());
        pipeline.add_transformer(transformer2, "_o2".to_string());

        let input = Bytes::from("data");
        let (output, _) = pipeline.execute(input).await.unwrap();

        // Should apply transformations in order
        assert_eq!(output, Bytes::from("t2_t1_data_o1_o2"));
    }

    #[tokio::test]
    async fn test_pipeline_empty_steps() {
        let pipeline: ProcessingPipeline<MockProcessor, MockTransformer> =
            ProcessingPipeline::new();
        let input = Bytes::from("data");
        let (output, metadata) = pipeline.execute(input.clone()).await.unwrap();

        // Empty pipeline should return input unchanged
        assert_eq!(output, input);
        assert_eq!(metadata, None);
    }
}
