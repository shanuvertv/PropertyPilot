//! Windows Service wrapper (PLAN.md D12): `renewal-server --service` is what
//! `sc create` points at. The service main starts the same tokio runtime as the
//! console mode and asks it to stop when the Service Control Manager says so.

#[cfg(windows)]
pub mod windows {
    use std::ffi::OsString;
    use std::sync::mpsc;
    use std::time::Duration;

    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    pub const SERVICE_NAME: &str = "PropertyPilotServer";

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> anyhow::Result<()> {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        Ok(())
    }

    fn service_main(_args: Vec<OsString>) {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handler = move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let Ok(status) = service_control_handler::register(SERVICE_NAME, handler) else {
            return;
        };
        let set = |state: ServiceState, code: u32| {
            let _ = status.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: state,
                controls_accepted: if state == ServiceState::Running {
                    ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
                } else {
                    ServiceControlAccept::empty()
                },
                exit_code: ServiceExitCode::Win32(code),
                checkpoint: 0,
                wait_hint: Duration::from_secs(10),
                process_id: None,
            });
        };
        set(ServiceState::StartPending, 0);
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(_) => {
                set(ServiceState::Stopped, 1);
                return;
            }
        };
        set(ServiceState::Running, 0);
        let outcome = rt.block_on(crate::run_server(async move {
            let _ = tokio::task::spawn_blocking(move || stop_rx.recv()).await;
        }));
        set(ServiceState::Stopped, if outcome.is_ok() { 0 } else { 1 });
    }
}
