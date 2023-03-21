use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use windows::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
use windows_service::{
    service::{
        Service, ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

use super::SystemServiceController;

pub struct WindowsServiceController {
    service_type: ServiceType,
    service_name: String,
    description: String,
    executable_path: PathBuf,
    uninstall_timeout: u64,
}

impl WindowsServiceController {
    pub fn new(
        service_type: ServiceType,
        service_name: String,
        description: String,
        executable_path: PathBuf,
        uninstall_timeout: u64,
    ) -> Self {
        Self {
            service_type,
            service_name,
            description,
            executable_path,
            uninstall_timeout,
        }
    }

    fn pause_resume(&self, pause: bool) -> Result<()> {
        let server_name = &self.service_name;
        let manager_access = ServiceManagerAccess::CONNECT;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
        let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::PAUSE_CONTINUE;
        let service = service_manager.open_service(server_name, service_access)?;

        let status = service.query_status()?;
        if pause {
            if status.current_state != ServiceState::Running {
                bail!("{} is {:?}", server_name, status.current_state)
            }
            println!("Pause {}", server_name);
            service.pause()?;
            self.wait(service, ServiceState::Paused)?;
        } else {
            if status.current_state != ServiceState::Paused {
                bail!("{} is {:?}", server_name, status.current_state)
            }
            println!("Resume {}", server_name);
            service.resume()?;
            self.wait(service, ServiceState::Running)?;
        }

        Ok(())
    }

    fn wait(&self, service: Service, state: ServiceState) -> Result<Service> {
        let start = Instant::now();
        let timeout = Duration::from_secs(2);
        while start.elapsed() < timeout {
            if service.query_status()?.current_state == state {
                println!("{:?}", state);
                return Ok(service);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        bail!("wait {:?} {:?} timeout", self.service_name, state)
    }
}

impl SystemServiceController for WindowsServiceController {
    fn install(&self, arguments: Vec<OsString>) -> Result<()> {
        let service_name = &self.service_name;
        let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
        let service_access = ServiceAccess::QUERY_STATUS;

        if let Ok(_) = service_manager.open_service(service_name, service_access) {
            bail!("{} has installed", service_name);
        }

        let service_info = ServiceInfo {
            name: service_name.into(),
            display_name: service_name.into(),
            service_type: self.service_type,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: self.executable_path.clone(),
            launch_arguments: arguments,
            dependencies: vec![],
            account_name: None,
            account_password: None,
        };
        let service =
            service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
        service.set_description(self.description.clone())?;
        println!("{} installed", service_name);

        Ok(())
    }

    fn start(&self, arguments: Vec<OsString>) -> Result<()> {
        let service_name = &self.service_name;
        let manager_access = ServiceManagerAccess::CONNECT;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
        let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START;
        let service = service_manager.open_service(service_name, service_access)?;

        let state = service.query_status()?.current_state;
        match state {
            ServiceState::Stopped => {
                println!("Start {}", service_name);
                service.start(&arguments)?;
            }
            _ => {
                bail!("{} is {:?}", service_name, state)
            }
        }
        self.wait(service, ServiceState::Running)?;

        Ok(())
    }

    fn restart(&self, arguments: Vec<OsString>) -> Result<()> {
        let service_name = &self.service_name;
        let manager_access = ServiceManagerAccess::CONNECT;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::START | ServiceAccess::STOP;
        let service = service_manager.open_service(service_name, service_access)?;

        if service.query_status()?.current_state != ServiceState::Stopped {
            println!("Stop {}", service_name);
            service.stop()?;
        }

        let service = self.wait(service, ServiceState::Stopped)?;

        println!("Start {}", service_name);
        service.start(&arguments)?;
        self.wait(service, ServiceState::Running)?;

        Ok(())
    }

    fn pause(&self) -> Result<()> {
        self.pause_resume(true)
    }

    fn resume(&self) -> Result<()> {
        self.pause_resume(false)
    }

    fn stop(&self) -> Result<()> {
        let service_name = &self.service_name;
        let manager_access = ServiceManagerAccess::CONNECT;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
        let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP;
        let service = service_manager.open_service(service_name, service_access)?;

        if service.query_status()?.current_state == ServiceState::Stopped {
            bail!("{} has stoped", service_name)
        }

        println!("Stop {}", service_name);
        service.stop()?;
        self.wait(service, ServiceState::Stopped)?;

        Ok(())
    }

    fn status(&self) -> Result<()> {
        let server_name = &self.service_name;
        let manager_access = ServiceManagerAccess::CONNECT;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
        let service_access = ServiceAccess::QUERY_STATUS;
        let service = match service_manager.open_service(server_name, service_access) {
            Ok(s) => s,
            Err(windows_service::Error::Winapi(e)) => {
                if Some(ERROR_SERVICE_DOES_NOT_EXIST.0 as _) == e.raw_os_error() {
                    bail!("{} not installed", server_name)
                }
                return Err(e.into());
            }
            Err(e) => {
                return Err(e.into());
            }
        };

        let status = service.query_status()?;
        println!("{:?}", status.current_state);

        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let service_name = &self.service_name;
        let manager_access = ServiceManagerAccess::CONNECT;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        let service = match service_manager.open_service(service_name, service_access) {
            Ok(s) => {
                if s.query_status()?.current_state == ServiceState::Running {
                    bail!("{} is {:?}", service_name, ServiceState::Running)
                }
                s
            }
            Err(windows_service::Error::Winapi(e)) => {
                if Some(ERROR_SERVICE_DOES_NOT_EXIST.0 as _) == e.raw_os_error() {
                    bail!("{} not installed", service_name)
                }
                return Err(e.into());
            }
            Err(e) => {
                return Err(e.into());
            }
        };

        service.delete()?;
        if service.query_status()?.current_state != ServiceState::Stopped {
            service.stop()?;
        }
        drop(service);

        let start = Instant::now();
        let timeout = Duration::from_secs(self.uninstall_timeout);
        while start.elapsed() < timeout {
            if let Err(windows_service::Error::Winapi(e)) =
                service_manager.open_service(service_name, ServiceAccess::QUERY_STATUS)
            {
                if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST.0 as _) {
                    println!("{} is deleted", service_name);
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        println!("{} is marked for deletion", service_name);

        Ok(())
    }
}
