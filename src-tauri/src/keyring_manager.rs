use keyring::Entry;

/// Service identifier for Windows Credential Manager.
const SERVICE: &str = "com.c3nx.quill-windows";

/// Key type for the Gemini API key.
pub const GEMINI_KEY: &str = "gemini-api-key";

/// Key type for the Claude API key.
pub const CLAUDE_KEY: &str = "claude-api-key";

/// Retrieve an API key from the credential store.
///
/// Returns `None` if the key is not found or cannot be read.
pub fn get_api_key(key_type: &str) -> Option<String> {
    let entry = Entry::new(SERVICE, key_type).ok()?;
    entry.get_password().ok()
}

/// Save an API key to the credential store.
pub fn save_api_key(key_type: &str, value: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, key_type)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    entry.set_password(value).map_err(|e| e.to_string())
}

/// Delete an API key from the credential store.
pub fn delete_api_key(key_type: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, key_type)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    entry.delete_credential().map_err(|e| e.to_string())
}

// =============================================================================
// Tauri commands -- thin wrappers around core functions
// =============================================================================

#[tauri::command]
pub fn get_gemini_key() -> Option<String> {
    get_api_key(GEMINI_KEY)
}

#[tauri::command]
pub fn save_gemini_key(key: String) -> Result<(), String> {
    save_api_key(GEMINI_KEY, &key)
}

#[tauri::command]
pub fn delete_gemini_key() -> Result<(), String> {
    delete_api_key(GEMINI_KEY)
}

#[tauri::command]
pub fn get_claude_key() -> Option<String> {
    get_api_key(CLAUDE_KEY)
}

#[tauri::command]
pub fn save_claude_key(key: String) -> Result<(), String> {
    save_api_key(CLAUDE_KEY, &key)
}

#[tauri::command]
pub fn delete_claude_key() -> Result<(), String> {
    delete_api_key(CLAUDE_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize credential store tests to avoid Windows Credential Manager
    // race conditions when multiple tests access the store concurrently.
    static CRED_LOCK: Mutex<()> = Mutex::new(());

    // =========================================================================
    // Unit tests: constants and Tauri command signatures
    // =========================================================================

    #[test]
    fn gemini_key_constant_is_correct() {
        assert_eq!(GEMINI_KEY, "gemini-api-key");
    }

    #[test]
    fn claude_key_constant_is_correct() {
        assert_eq!(CLAUDE_KEY, "claude-api-key");
    }

    #[test]
    fn service_constant_is_correct() {
        assert_eq!(SERVICE, "com.c3nx.quill-windows");
    }

    // =========================================================================
    // Integration tests: real Windows Credential Manager round-trip.
    // These tests access the real credential store and clean up after themselves.
    // They are serialized via CRED_LOCK to avoid race conditions.
    // =========================================================================

    #[test]
    fn get_nonexistent_key_returns_none() {
        let _lock = CRED_LOCK.lock().unwrap();
        let result = get_api_key("nonexistent-key-that-should-never-exist");
        assert!(result.is_none());
    }

    #[test]
    fn save_get_delete_round_trip() {
        let _lock = CRED_LOCK.lock().unwrap();
        let key = "test-quill-round-trip";
        let test_value = "sk-test-round-trip-value-12345";

        // Save
        let save_result = save_api_key(key, test_value);
        assert!(save_result.is_ok(), "save_api_key failed: {:?}", save_result);

        // Get -- should retrieve the saved value
        let retrieved = get_api_key(key);
        assert_eq!(retrieved, Some(test_value.to_string()));

        // Delete -- cleanup
        let delete_result = delete_api_key(key);
        assert!(delete_result.is_ok(), "delete_api_key failed: {:?}", delete_result);

        // Verify deletion
        let after_delete = get_api_key(key);
        assert!(after_delete.is_none(), "Key should be None after deletion");
    }

    #[test]
    fn save_overwrites_existing_key() {
        let _lock = CRED_LOCK.lock().unwrap();
        let key = "test-quill-overwrite";
        let first_value = "first-value-abc";
        let second_value = "second-value-xyz";

        // Save first value
        save_api_key(key, first_value).expect("first save failed");

        // Overwrite with second value
        save_api_key(key, second_value).expect("second save failed");

        // Should return the second value
        let retrieved = get_api_key(key);
        assert_eq!(retrieved, Some(second_value.to_string()));

        // Cleanup
        delete_api_key(key).expect("cleanup delete failed");
    }

    #[test]
    fn delete_nonexistent_key_returns_error() {
        let _lock = CRED_LOCK.lock().unwrap();
        let result = delete_api_key("nonexistent-key-for-delete-test");
        assert!(result.is_err());
    }

    #[test]
    fn save_and_get_empty_string() {
        let _lock = CRED_LOCK.lock().unwrap();
        let key = "test-quill-empty-value";

        // Save empty string
        save_api_key(key, "").expect("save empty failed");

        // Get -- should return Some("")
        let retrieved = get_api_key(key);
        assert_eq!(retrieved, Some(String::new()));

        // Cleanup
        delete_api_key(key).expect("cleanup delete failed");
    }

    #[test]
    fn save_and_get_special_characters() {
        let _lock = CRED_LOCK.lock().unwrap();
        let key = "test-quill-special-chars";
        let special_value = "sk-abc123!@#$%^&*()_+-=[]{}|;':\",./<>?";

        save_api_key(key, special_value).expect("save special chars failed");

        let retrieved = get_api_key(key);
        assert_eq!(retrieved, Some(special_value.to_string()));

        // Cleanup
        delete_api_key(key).expect("cleanup delete failed");
    }
}
