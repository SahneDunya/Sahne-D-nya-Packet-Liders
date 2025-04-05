use fluent_bundle::{FluentBundle, FluentResource, LocalizedString, FluentValue};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;
use thiserror::Error;

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;

#[derive(Error, Debug)]
pub enum I18nError {
    #[error("Failed to read directory: {0}")]
    DirectoryReadError(String),
    #[error("Failed to read file: {0}")]
    FileReadError(String),
    #[error("Failed to parse language identifier: {0}")]
    LanguageIdentifierParseError(String),
    #[error("Failed to parse Fluent resource: {0}")]
    FluentResourceParseError(String),
    #[error("Failed to add resource to bundle: {0}")]
    ResourceAdditionError(String),
    #[error("Language bundle not found: {0}")]
    BundleNotFound(String),
    #[error("Message not found: {0}")]
    MessageNotFound(String),
    #[error("Message value not found: {0}")]
    MessageValueError(String),
    #[error("Message evaluation error: {0}")]
    MessageEvaluationError(String),
    #[error("Sahne64 File System Error: {0}")]
    Sahne64FileSystemError(#[from] crate::SahneError), // Assuming SahneError is in the root or accessible
}

pub struct I18n {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
    default_language: Option<LanguageIdentifier>, // Added default language
}

impl I18n {
    // Modified to accept a HashMap of language identifiers to a list of file paths
    pub fn new(locale_files: HashMap<LanguageIdentifier, Vec<&str>>, default_language: Option<LanguageIdentifier>) -> Result<Self, I18nError> {
        let mut bundles = HashMap::new();
        for (lang_id, file_paths) in locale_files {
            let mut bundle = FluentBundle::new(vec![lang_id.clone()]);
            for file_path in file_paths {
                if file_path.ends_with(".ftl") {
                    match fs::open(file_path, fs::O_RDONLY) {
                        Ok(fd) => {
                            let mut buffer = Vec::new();
                            let mut read_buffer = [0u8; 128]; // Read in chunks
                            loop {
                                match fs::read(fd, &mut read_buffer) {
                                    Ok(bytes_read) => {
                                        if bytes_read == 0 {
                                            break;
                                        }
                                        buffer.extend_from_slice(&read_buffer[..bytes_read]);
                                    }
                                    Err(e) => {
                                        fs::close(fd).unwrap_or_default(); // Attempt to close on error
                                        return Err(I18nError::Sahne64FileSystemError(e));
                                    }
                                }
                            }
                            fs::close(fd).unwrap_or_default();

                            match String::from_utf8(buffer) {
                                Ok(source) => {
                                    match FluentResource::try_new(source) {
                                        Ok(resource) => {
                                            if let Err(errors) = bundle.add_resource(resource) {
                                                for error in errors {
                                                    eprintln!("Error adding resource to bundle: {:?}", error);
                                                }
                                                return Err(I18nError::ResourceAdditionError(lang_id.to_string()));
                                            }
                                        }
                                        Err((_, e)) => return Err(I18nError::FluentResourceParseError(e.to_string())),
                                    }
                                }
                                Err(e) => return Err(I18nError::FileReadError(e.to_string())),
                            }
                        }
                        Err(e) => return Err(I18nError::Sahne64FileSystemError(e)),
                    }
                }
            }
            bundles.insert(lang_id, bundle);
        }
        Ok(I18n { bundles, default_language })
    }

    pub fn get_message(
        &self,
        lang_id: &LanguageIdentifier,
        key: &str,
        args: Option<&HashMap<&str, FluentValue>>,
    ) -> Result<LocalizedString, I18nError> {
        // Try to get the bundle for the requested language
        let bundle = self.bundles.get(lang_id).or_else(|| {
            // If not found, try to use the default language bundle if it's set
            self.default_language.as_ref().and_then(|default_lang| self.bundles.get(default_lang))
        }).ok_or_else(|| I18nError::BundleNotFound(lang_id.to_string()))?;

        let message = bundle.get_message(key).ok_or_else(|| I18nError::MessageNotFound(key.to_string()))?;
        let pattern = message.value().ok_or_else(|| I18nError::MessageValueError(key.to_string()))?;
        pattern.evaluate(bundle, args).map_err(|e| I18nError::MessageEvaluationError(e.to_string()))
    }
}