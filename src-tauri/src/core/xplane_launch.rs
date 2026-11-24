use std::{io, path::Path};

use crate::error::{ApiError, ApiErrorCode};

/// Translate errors from the Windows process-launch boundary.
/// This module is only included on Windows (or in cross-platform unit tests).
pub(crate) fn windows_launch_error(executable: &Path, error: io::Error) -> String {
    match error.raw_os_error() {
        Some(740) => format!("ELEVATION_REQUIRED:{}", executable.display()),
        Some(193) => "Selected X-Plane executable is not a valid Windows executable. Check that the X-Plane path points to the real X-Plane installation."
            .to_string(),
        Some(4551) => {
            // ERROR_SYSTEM_INTEGRITY_POLICY_VIOLATION is not specific to Smart App Control.
            // Diagnose the actual blocked file/policy via the enforced-policy event:
            // https://learn.microsoft.com/windows/apps/develop/smart-app-control/test-your-app-with-smart-app-control#checking-event-logs
            let mut api_error = ApiError::with_details(
                ApiErrorCode::ApplicationControlBlocked,
                format!("Failed to launch X-Plane: {error}"),
                // This is the launch target, not necessarily the file identified by event 3077.
                executable.to_string_lossy().into_owned(),
            );
            api_error.operation = Some("launch_xplane".to_string());
            api_error.user_action = Some(
                "In Event Viewer, check Microsoft-Windows-CodeIntegrity/Operational event 3077 at the time of failure to identify the blocked file and policy. Contact the file's publisher or your system administrator to verify its source and signature. Keep Windows security protections enabled."
                    .to_string(),
            );
            serde_json::to_string(&api_error).unwrap_or_else(|_| api_error.to_string())
        }
        _ => format!("Failed to launch X-Plane: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ApiError, ApiErrorOrigin};

    const EXECUTABLE: &str = r#"D:\Flight Sims\模拟飞行\X-Plane "Test"\X-Plane.exe"#;

    #[test]
    fn application_control_block_preserves_structured_diagnostics() {
        let error = io::Error::from_raw_os_error(4551);
        let original_message = error.to_string();
        let result = windows_launch_error(Path::new(EXECUTABLE), error);
        let serialized: serde_json::Value =
            serde_json::from_str(&result).expect("4551 must return a structured API error");
        assert_eq!(serialized["code"], "application_control_blocked");
        assert_eq!(serialized["operation"], "launch_xplane");
        assert_eq!(serialized["origin"], "environment");
        assert_eq!(serialized["reportable"], false);
        assert_eq!(serialized["details"], EXECUTABLE);

        let error: ApiError = serde_json::from_str(&result).unwrap();
        assert_eq!(error.origin, ApiErrorOrigin::Environment);
        assert!(error.message.contains(&original_message));
        assert!(error.message.contains("4551"));
        let action = error.user_action.unwrap();
        assert!(action.contains("Microsoft-Windows-CodeIntegrity/Operational"));
        assert!(action.contains("3077"));
    }

    #[test]
    fn elevation_required_keeps_its_existing_frontend_contract() {
        assert_eq!(
            windows_launch_error(Path::new(EXECUTABLE), io::Error::from_raw_os_error(740)),
            format!("ELEVATION_REQUIRED:{EXECUTABLE}")
        );
    }

    #[test]
    fn bad_executable_format_keeps_installation_guidance() {
        assert_eq!(
            windows_launch_error(Path::new(EXECUTABLE), io::Error::from_raw_os_error(193)),
            "Selected X-Plane executable is not a valid Windows executable. Check that the X-Plane path points to the real X-Plane installation."
        );
    }

    #[test]
    fn other_os_errors_keep_their_original_diagnostics() {
        for code in [2, 5, 4550, 4552] {
            let error = io::Error::from_raw_os_error(code);
            let original_message = error.to_string();
            assert_eq!(
                windows_launch_error(Path::new(EXECUTABLE), error),
                format!("Failed to launch X-Plane: {original_message}")
            );
        }
    }

    #[test]
    fn error_text_without_a_raw_windows_code_is_not_reclassified() {
        for message in [
            "Une stratégie de contrôle d’application a bloqué ce fichier. (os error 4551)",
            "An application control policy has blocked this file. (os error 4551)",
            "740: requires elevation; 193: invalid executable",
        ] {
            let error = io::Error::other(message);
            assert_eq!(error.raw_os_error(), None);
            assert_eq!(
                windows_launch_error(Path::new(EXECUTABLE), error),
                format!("Failed to launch X-Plane: {message}")
            );
        }
    }
}
