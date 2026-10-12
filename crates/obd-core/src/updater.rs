use crate::error::ObdError;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Details about an available update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// Currently running application version.
    pub current_version: String,
    /// Latest release version discovered on GitHub.
    pub latest_version: String,
    /// Whether latest_version > current_version according to SemVer.
    pub update_available: bool,
    /// Release title or changelog header.
    pub release_title: String,
    /// Changelog / release description markdown.
    pub release_notes: String,
    /// Link to view release on GitHub.
    pub html_url: String,
    /// Direct download link to the Windows .msi installer asset, if present.
    pub msi_download_url: Option<String>,
    /// Direct download link to the Windows .exe asset, if present.
    pub exe_download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Normalizes tag strings (e.g. "v0.2.0" or "0.2.0") into SemVer Version.
pub fn parse_clean_semver(tag: &str) -> Result<Version, ObdError> {
    let clean = tag.trim().trim_start_matches('v').trim_start_matches('V');
    Version::parse(clean)
        .map_err(|e| ObdError::ParseError(format!("Failed to parse version '{tag}': {e}")))
}

/// Checks for newer releases on GitHub for `bangsmackpow/odb2mon`.
pub async fn check_for_updates(current_version_str: &str) -> Result<UpdateInfo, ObdError> {
    let current_ver = parse_clean_semver(current_version_str)?;
    let api_url = "https://api.github.com/repos/bangsmackpow/odb2mon/releases/latest";

    let client = reqwest::Client::builder()
        .user_agent("odb2mon-update-checker")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| ObdError::ConnectionFailed(e.to_string()))?;

    let response = client
        .get(api_url)
        .send()
        .await
        .map_err(|e| ObdError::ConnectionFailed(format!("Update check request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(ObdError::Protocol(format!(
            "GitHub API responded with HTTP status {}",
            response.status()
        )));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| ObdError::ParseError(format!("Failed to parse release JSON: {e}")))?;

    parse_release_into_update_info(current_ver, release)
}

fn parse_release_into_update_info(
    current_ver: Version,
    release: GitHubRelease,
) -> Result<UpdateInfo, ObdError> {
    let latest_ver = parse_clean_semver(&release.tag_name)?;
    let update_available = latest_ver > current_ver;

    let mut msi_url = None;
    let mut exe_url = None;

    for asset in release.assets {
        let lower = asset.name.to_lowercase();
        if lower.ends_with(".msi") {
            msi_url = Some(asset.browser_download_url);
        } else if lower.ends_with(".exe") {
            exe_url = Some(asset.browser_download_url);
        }
    }

    Ok(UpdateInfo {
        current_version: current_ver.to_string(),
        latest_version: latest_ver.to_string(),
        update_available,
        release_title: release.name.unwrap_or_else(|| release.tag_name.clone()),
        release_notes: release.body.unwrap_or_default(),
        html_url: release.html_url,
        msi_download_url: msi_url,
        exe_download_url: exe_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_semver() {
        assert_eq!(
            parse_clean_semver("v1.2.3").unwrap(),
            Version::parse("1.2.3").unwrap()
        );
        assert_eq!(
            parse_clean_semver("0.1.0").unwrap(),
            Version::parse("0.1.0").unwrap()
        );
    }

    #[test]
    fn test_update_available_detection() {
        let current = Version::parse("0.1.0").unwrap();
        let release = GitHubRelease {
            tag_name: "v0.2.0".into(),
            name: Some("Release 0.2.0 - Added Live Gauges".into()),
            body: Some("Highlights:\n* Added MSI installer support\n* Live gauges".into()),
            html_url: "https://github.com/bangsmackpow/odb2mon/releases/tag/v0.2.0".into(),
            assets: vec![
                GitHubAsset {
                    name: "odb2mon-0.2.0-x64.msi".into(),
                    browser_download_url: "https://github.com/bangsmackpow/odb2mon/releases/download/v0.2.0/odb2mon-0.2.0-x64.msi".into(),
                },
                GitHubAsset {
                    name: "odb2mon.exe".into(),
                    browser_download_url: "https://github.com/bangsmackpow/odb2mon/releases/download/v0.2.0/odb2mon.exe".into(),
                },
            ],
        };

        let info = parse_release_into_update_info(current, release).unwrap();
        assert!(info.update_available);
        assert_eq!(info.current_version, "0.1.0");
        assert_eq!(info.latest_version, "0.2.0");
        assert!(info.msi_download_url.is_some());
        assert!(info.exe_download_url.is_some());
    }

    #[test]
    fn test_already_up_to_date() {
        let current = Version::parse("0.2.0").unwrap();
        let release = GitHubRelease {
            tag_name: "v0.2.0".into(),
            name: Some("v0.2.0".into()),
            body: None,
            html_url: "https://github.com/bangsmackpow/odb2mon/releases/tag/v0.2.0".into(),
            assets: vec![],
        };

        let info = parse_release_into_update_info(current, release).unwrap();
        assert!(!info.update_available);
    }
}
