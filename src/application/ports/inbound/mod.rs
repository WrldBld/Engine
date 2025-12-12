//! Inbound ports - Interfaces that the application exposes to the outside world

pub mod use_cases;

pub use use_cases::{
    // Error types
    UseCaseError,
    // World use cases
    CreateWorldRequest, WorldSummaryDto, WorldDto, ManageWorldUseCase,
    // Character use cases
    CreateCharacterRequest, CharacterSummaryDto, ManageCharacterUseCase,
    // Location use cases
    CreateLocationRequest, LocationSummaryDto, ManageLocationUseCase,
    // Scene use cases
    SceneSummaryDto, ManageSceneUseCase,
};
