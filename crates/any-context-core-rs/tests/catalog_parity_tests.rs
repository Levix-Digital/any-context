//! Guards against divergence between the installer catalog and the Core catalog (ADR-130, D13)
//! until both are unified into a single source in Frente C.

use any_context_core_rs::ingestion::{OnnxModelManager, DEFAULT_RERANKER_ID};

fn installer_files() -> Vec<actx_installer::hardware::ModelDownloadSpec> {
    actx_installer::hardware::ALL_MODELS
        .iter()
        .chain(actx_installer::hardware::RERANKER_FILES.iter())
        .copied()
        .collect()
}

#[test]
fn test_installer_catalog_matches_core() {
    for inst in installer_files() {
        let spec = OnnxModelManager::find_spec(inst.id)
            .unwrap_or_else(|| panic!("installer model '{}' missing from the Core catalog", inst.id));
        let core_file = spec
            .files
            .iter()
            .find(|f| f.file_name == inst.file_name)
            .unwrap_or_else(|| panic!("installer file '{}' missing from Core spec '{}'", inst.file_name, spec.id));
        assert_eq!(core_file.url, inst.download_url, "{}", inst.file_name);
        assert_eq!(core_file.size_bytes, inst.size_bytes, "{}", inst.file_name);
        assert_eq!(core_file.sha256, inst.sha256, "{}", inst.file_name);
    }
}

#[test]
fn test_installer_has_no_placeholder_or_planned_downloads() {
    for inst in installer_files() {
        assert!(!inst.download_url.contains("anycontext/models"), "{}", inst.download_url);
        assert!(!inst.download_url.contains("gemma-4"), "{}", inst.download_url);
        assert!(!inst.download_url.contains("/resolve/main/"), "{} must pin a revision", inst.download_url);
        assert_eq!(inst.sha256.len(), 64);
    }
    for tier in [
        actx_installer::hardware::InstallerProfileTier::SovereignUltra,
        actx_installer::hardware::InstallerProfileTier::WorkstationPro,
        actx_installer::hardware::InstallerProfileTier::AirGappedPower,
        actx_installer::hardware::InstallerProfileTier::SmartBalanced,
    ] {
        assert!(tier.target_models().iter().all(|m| !m.id.starts_with("gemma")));
        let auto: Vec<&str> = tier.auto_provisioned_models().iter().map(|m| m.id).collect();
        assert!(auto.iter().all(|id| *id == DEFAULT_RERANKER_ID), "{:?}", auto);
        assert_eq!(auto.len(), 2, "model + tokenizer");
    }
    assert!(actx_installer::hardware::InstallerProfileTier::Lite.auto_provisioned_models().is_empty());
}
