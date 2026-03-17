/// System keyring integration for proxy credentials.
///
/// Uses the platform's secure credential storage (macOS Keychain,
/// Windows Credential Manager, Linux Secret Service).

const SERVICE_NAME: &str = "ratiomaster";

/// Loads a proxy password from the system keyring.
pub fn load_password(username: &str) -> Option<String> {
    let entry = keyring::Entry::new(SERVICE_NAME, username).ok()?;
    entry.get_password().ok()
}

/// Stores a proxy password in the system keyring.
pub fn store_password(username: &str, password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, username).map_err(|e| e.to_string())?;
    entry.set_password(password).map_err(|e| e.to_string())
}

/// Deletes a proxy password from the system keyring.
pub fn delete_password(username: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, username).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
