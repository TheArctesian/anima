//! The capability tags of the catalogue (LIBM-008). A tag says what a model can do; it is not
//! evidence that a runtime of Anima runs the model, and it selects no runtime (LIBM-009).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

/// One tag of LIBM-008. A subtype is a tag of its own, not a serving layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Enum)]
#[graphql(name = "ModelCapability")]
pub enum Capability {
    // Text
    Chat,
    Coding,
    Reasoning,
    Translation,
    Summarization,
    TextClassification,
    StructuredExtraction,
    TokenClassification,
    // Image
    ImageGeneration,
    ImageEditing,
    Upscaling,
    ImageClassification,
    // Video
    VideoGeneration,
    VideoEditing,
    VideoUnderstanding,
    VideoClassification,
    // Audio and music
    MusicGeneration,
    AudioGeneration,
    AudioUnderstanding,
    AudioClassification,
    AudioToAudio,
    // Speech
    Transcription,
    SpokenTranslation,
    TextToSpeech,
    ConversationalVoice,
    // Vision and documents
    ImageUnderstanding,
    Ocr,
    DocumentFieldExtraction,
    DocumentQuestionAnswering,
    VisualDocumentRetrieval,
    ObjectDetection,
    ImageSegmentation,
    DepthEstimation,
    PoseDetection,
    // Search
    Embeddings,
    Reranking,
    // Tabular
    TabularClassification,
    TabularRegression,
    // Reinforcement learning: a tag of the catalogue only
    Policy,
    // Later: 3D object, mesh and scene generation
    ThreeDimensionalGeneration,
}

/// Each tag with the text stored in `compute.models.capabilities`.
const STORED: [(Capability, &str); 40] = [
    (Capability::Chat, "chat"),
    (Capability::Coding, "coding"),
    (Capability::Reasoning, "reasoning"),
    (Capability::Translation, "translation"),
    (Capability::Summarization, "summarization"),
    (Capability::TextClassification, "text_classification"),
    (Capability::StructuredExtraction, "structured_extraction"),
    (Capability::TokenClassification, "token_classification"),
    (Capability::ImageGeneration, "image_generation"),
    (Capability::ImageEditing, "image_editing"),
    (Capability::Upscaling, "upscaling"),
    (Capability::ImageClassification, "image_classification"),
    (Capability::VideoGeneration, "video_generation"),
    (Capability::VideoEditing, "video_editing"),
    (Capability::VideoUnderstanding, "video_understanding"),
    (Capability::VideoClassification, "video_classification"),
    (Capability::MusicGeneration, "music_generation"),
    (Capability::AudioGeneration, "audio_generation"),
    (Capability::AudioUnderstanding, "audio_understanding"),
    (Capability::AudioClassification, "audio_classification"),
    (Capability::AudioToAudio, "audio_to_audio"),
    (Capability::Transcription, "transcription"),
    (Capability::SpokenTranslation, "spoken_translation"),
    (Capability::TextToSpeech, "text_to_speech"),
    (Capability::ConversationalVoice, "conversational_voice"),
    (Capability::ImageUnderstanding, "image_understanding"),
    (Capability::Ocr, "ocr"),
    (
        Capability::DocumentFieldExtraction,
        "document_field_extraction",
    ),
    (
        Capability::DocumentQuestionAnswering,
        "document_question_answering",
    ),
    (
        Capability::VisualDocumentRetrieval,
        "visual_document_retrieval",
    ),
    (Capability::ObjectDetection, "object_detection"),
    (Capability::ImageSegmentation, "image_segmentation"),
    (Capability::DepthEstimation, "depth_estimation"),
    (Capability::PoseDetection, "pose_detection"),
    (Capability::Embeddings, "embeddings"),
    (Capability::Reranking, "reranking"),
    (Capability::TabularClassification, "tabular_classification"),
    (Capability::TabularRegression, "tabular_regression"),
    (Capability::Policy, "policy"),
    (
        Capability::ThreeDimensionalGeneration,
        "three_dimensional_generation",
    ),
];

/// Each Hugging Face task (`pipeline_tag`) with the tags it gives.
const TASKS: [(&str, &[Capability]); 33] = [
    ("text-generation", &[Capability::Chat]),
    (
        "image-text-to-text",
        &[Capability::Chat, Capability::ImageUnderstanding],
    ),
    (
        "video-text-to-text",
        &[Capability::Chat, Capability::VideoUnderstanding],
    ),
    (
        "audio-text-to-text",
        &[Capability::Chat, Capability::AudioUnderstanding],
    ),
    ("feature-extraction", &[Capability::Embeddings]),
    ("sentence-similarity", &[Capability::Embeddings]),
    ("text-ranking", &[Capability::Reranking]),
    ("translation", &[Capability::Translation]),
    ("summarization", &[Capability::Summarization]),
    ("text-classification", &[Capability::TextClassification]),
    ("token-classification", &[Capability::TokenClassification]),
    ("text-to-image", &[Capability::ImageGeneration]),
    ("image-to-image", &[Capability::ImageEditing]),
    ("image-classification", &[Capability::ImageClassification]),
    ("text-to-video", &[Capability::VideoGeneration]),
    ("video-classification", &[Capability::VideoClassification]),
    ("text-to-audio", &[Capability::AudioGeneration]),
    ("audio-classification", &[Capability::AudioClassification]),
    ("audio-to-audio", &[Capability::AudioToAudio]),
    ("automatic-speech-recognition", &[Capability::Transcription]),
    ("text-to-speech", &[Capability::TextToSpeech]),
    ("image-to-text", &[Capability::ImageUnderstanding]),
    (
        "document-question-answering",
        &[Capability::DocumentQuestionAnswering],
    ),
    (
        "table-question-answering",
        &[Capability::DocumentQuestionAnswering],
    ),
    (
        "visual-document-retrieval",
        &[Capability::VisualDocumentRetrieval],
    ),
    ("object-detection", &[Capability::ObjectDetection]),
    ("image-segmentation", &[Capability::ImageSegmentation]),
    ("depth-estimation", &[Capability::DepthEstimation]),
    ("keypoint-detection", &[Capability::PoseDetection]),
    (
        "tabular-classification",
        &[Capability::TabularClassification],
    ),
    ("tabular-regression", &[Capability::TabularRegression]),
    ("reinforcement-learning", &[Capability::Policy]),
    ("image-to-3d", &[Capability::ThreeDimensionalGeneration]),
];

/// Repository tags that add a tag of their own to the task's tags.
const REPOSITORY_TAGS: [(&str, Capability); 4] = [
    ("code", Capability::Coding),
    ("reasoning", Capability::Reasoning),
    ("ocr", Capability::Ocr),
    ("text-to-3d", Capability::ThreeDimensionalGeneration),
];

impl Capability {
    /// The text stored for the tag.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        STORED
            .iter()
            .find(|(tag, _)| *tag == self)
            .map_or("", |(_, text)| text)
    }

    /// The tag whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        STORED
            .iter()
            .find(|(_, stored)| *stored == text)
            .map(|(tag, _)| *tag)
    }
}

/// The tags that the repository's task and its tags give, sorted and without repeats
/// (LIBM-005: the hub reads them, it does not ask the operator).
#[must_use]
pub fn from_repository(task: Option<&str>, tags: &[String]) -> Vec<Capability> {
    let mut found: Vec<Capability> = TASKS
        .iter()
        .filter(|(name, _)| Some(*name) == task)
        .flat_map(|(_, given)| given.iter().copied())
        .collect();
    for (tag, capability) in REPOSITORY_TAGS {
        if tags.iter().any(|each| each == tag) {
            found.push(capability);
        }
    }
    found.sort();
    found.dedup();
    found
}
