//! Consumer-view compile checks for the Experimental presenter surface.

use std::time::Duration;

use kinetik_ui_core::Color;
use kinetik_ui_vello_winit::{
    AaConfig, InvalidColorChannel, PresenterDevice, PresenterDeviceScope, PresenterGpuError,
    PresenterGpuErrorKind, VelloAttachOutcome, VelloAttachmentStatus, VelloPresentReport,
    VelloPresentStatus, VelloPresenterConfig, VelloPresenterError, VelloPresenterStatus,
    VelloRecoveryKind, VelloRecoveryOutcome, VelloRedrawGuidance, VelloResizeOutcome,
    VelloSuspendOutcome, VelloWindowPresenter, wgpu,
};

fn same_type<T>(_: Option<T>, _: Option<T>) {}

#[test]
fn every_qualified_public_presenter_type_is_importable_without_a_window_or_gpu() {
    let type_names = [
        std::any::type_name::<VelloWindowPresenter>(),
        std::any::type_name::<VelloPresenterConfig>(),
        std::any::type_name::<PresenterDeviceScope>(),
        std::any::type_name::<PresenterDevice<'static>>(),
        std::any::type_name::<PresenterGpuError>(),
        std::any::type_name::<PresenterGpuErrorKind>(),
        std::any::type_name::<InvalidColorChannel>(),
        std::any::type_name::<VelloPresenterError>(),
        std::any::type_name::<VelloAttachmentStatus>(),
        std::any::type_name::<VelloPresenterStatus>(),
        std::any::type_name::<VelloAttachOutcome>(),
        std::any::type_name::<VelloSuspendOutcome>(),
        std::any::type_name::<VelloResizeOutcome>(),
        std::any::type_name::<VelloRecoveryKind>(),
        std::any::type_name::<VelloRecoveryOutcome>(),
        std::any::type_name::<VelloPresentStatus>(),
        std::any::type_name::<VelloPresentReport>(),
        std::any::type_name::<VelloRedrawGuidance>(),
    ];
    assert!(type_names.iter().all(|name| !name.is_empty()));
}

#[test]
fn wgpu_and_antialiasing_reexports_keep_the_exact_vello_identity() {
    same_type::<wgpu::Device>(None, None::<vello::wgpu::Device>);
    same_type::<wgpu::Queue>(None, None::<vello::wgpu::Queue>);
    same_type::<AaConfig>(None, None::<vello::AaConfig>);
}

#[test]
fn detached_construction_and_private_config_builders_are_gpu_free() {
    let config = VelloPresenterConfig::new()
        .with_antialiasing_method(AaConfig::Area)
        .with_base_color(Color::rgba(0.1, 0.2, 0.3, 1.0))
        .unwrap()
        .with_timeout_retry(Duration::from_millis(25))
        .unwrap();
    let presenter = VelloWindowPresenter::new(config).unwrap();

    assert_eq!(
        presenter.status().attachment(),
        VelloAttachmentStatus::Detached
    );
    assert!(presenter.status().device_scope().is_none());
}

#[test]
fn non_exhaustive_statuses_are_matched_with_a_fallback() {
    fn describe(status: VelloPresentStatus) -> &'static str {
        match status {
            VelloPresentStatus::Presented => "presented",
            _ => "other",
        }
    }

    assert_eq!(describe(VelloPresentStatus::Timeout), "other");
}
