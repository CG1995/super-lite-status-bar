use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GpuInfo {
    pub name: Option<String>,
    pub usage_percent: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub temperature_celsius: Option<f32>,
    pub available: bool,
    pub source: Option<String>,
}

#[derive(Debug)]
pub struct GpuSampler {
    #[cfg(target_os = "windows")]
    nvml_sampler: nvml::NvmlSampler,
    #[cfg(target_os = "macos")]
    cached_name: Option<String>,
}

impl Default for GpuSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuSampler {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            nvml_sampler: nvml::NvmlSampler::new(),
            #[cfg(target_os = "macos")]
            cached_name: None,
        }
    }

    pub fn sample(&mut self) -> GpuInfo {
        #[cfg(target_os = "windows")]
        {
            if let Some(info) = self.nvml_sampler.sample() {
                return info;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if self.cached_name.is_none() {
                self.cached_name = sample_macos_gpu_name();
            }
            if let Some(name) = &self.cached_name {
                return GpuInfo {
                    name: Some(name.clone()),
                    available: true,
                    source: Some("system_profiler".to_string()),
                    ..GpuInfo::default()
                };
            }
        }

        GpuInfo::default()
    }
}

#[cfg(target_os = "windows")]
#[allow(clippy::manual_c_str_literals)]
mod nvml {
    use std::ffi::{c_char, c_void, CStr};
    use std::time::{Duration, Instant};

    type NvmlReturn = i32;
    const NVML_SUCCESS: NvmlReturn = 0;
    const NVML_TEMPERATURE_GPU: u32 = 0;

    type NvmlDevice = *mut c_void;

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct NvmlUtilization {
        pub gpu: u32,
        pub memory: u32,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct NvmlMemory {
        pub total: u64,
        pub free: u64,
        pub used: u64,
    }

    type NvmlInitFn = unsafe extern "C" fn() -> NvmlReturn;
    type NvmlShutdownFn = unsafe extern "C" fn() -> NvmlReturn;
    type NvmlDeviceGetCountFn = unsafe extern "C" fn(*mut u32) -> NvmlReturn;
    type NvmlDeviceGetHandleByIndexFn = unsafe extern "C" fn(u32, *mut NvmlDevice) -> NvmlReturn;
    type NvmlDeviceGetNameFn = unsafe extern "C" fn(NvmlDevice, *mut c_char, u32) -> NvmlReturn;
    type NvmlDeviceGetUtilizationRatesFn =
        unsafe extern "C" fn(NvmlDevice, *mut NvmlUtilization) -> NvmlReturn;
    type NvmlDeviceGetMemoryInfoFn =
        unsafe extern "C" fn(NvmlDevice, *mut NvmlMemory) -> NvmlReturn;
    type NvmlDeviceGetTemperatureFn = unsafe extern "C" fn(NvmlDevice, u32, *mut u32) -> NvmlReturn;

    extern "system" {
        fn LoadLibraryA(lp_lib_file_name: *const c_char) -> *mut c_void;
        fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const c_char) -> *mut c_void;
        fn FreeLibrary(h_lib_module: *mut c_void) -> i32;
    }

    struct NvmlLoaded {
        module: *mut c_void,
        device: NvmlDevice,
        name: String,
        fn_shutdown: NvmlShutdownFn,
        fn_get_util: NvmlDeviceGetUtilizationRatesFn,
        fn_get_mem: NvmlDeviceGetMemoryInfoFn,
        fn_get_temp: NvmlDeviceGetTemperatureFn,
    }

    unsafe impl Send for NvmlLoaded {}

    impl Drop for NvmlLoaded {
        fn drop(&mut self) {
            unsafe {
                (self.fn_shutdown)();
                FreeLibrary(self.module);
            }
        }
    }

    pub struct NvmlSampler {
        loaded: Option<NvmlLoaded>,
        last_attempt: Option<Instant>,
    }

    unsafe impl Send for NvmlSampler {}

    impl std::fmt::Debug for NvmlSampler {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("NvmlSampler")
                .field("is_loaded", &self.loaded.is_some())
                .field("last_attempt", &self.last_attempt)
                .finish()
        }
    }

    const RETRY_BACKOFF: Duration = Duration::from_secs(30);

    unsafe fn get_proc<T>(module: *mut c_void, name: &CStr) -> Option<T> {
        let proc = GetProcAddress(module, name.as_ptr());
        if proc.is_null() {
            None
        } else {
            Some(std::mem::transmute_copy(&proc))
        }
    }

    impl NvmlSampler {
        pub fn new() -> Self {
            Self {
                loaded: None,
                last_attempt: None,
            }
        }

        pub fn sample(&mut self) -> Option<super::GpuInfo> {
            if self.loaded.is_none() {
                if let Some(last) = self.last_attempt {
                    if last.elapsed() < RETRY_BACKOFF {
                        return None;
                    }
                }
                self.last_attempt = Some(Instant::now());
                self.loaded = Self::try_init();
            }

            let loaded = self.loaded.as_mut()?;
            unsafe {
                let mut util = NvmlUtilization::default();
                let mut mem = NvmlMemory::default();
                let mut temp: u32 = 0;

                let util_res = (loaded.fn_get_util)(loaded.device, &mut util);
                let mem_res = (loaded.fn_get_mem)(loaded.device, &mut mem);
                let temp_res = (loaded.fn_get_temp)(loaded.device, NVML_TEMPERATURE_GPU, &mut temp);

                // If calls fail (e.g. eGPU disconnected or device lost), drop loaded instance and back off
                if util_res != NVML_SUCCESS && mem_res != NVML_SUCCESS && temp_res != NVML_SUCCESS {
                    self.loaded = None;
                    self.last_attempt = Some(Instant::now());
                    return None;
                }

                Some(super::GpuInfo {
                    name: Some(loaded.name.clone()),
                    usage_percent: if util_res == NVML_SUCCESS {
                        Some(util.gpu as f32)
                    } else {
                        None
                    },
                    memory_used_bytes: if mem_res == NVML_SUCCESS {
                        Some(mem.used)
                    } else {
                        None
                    },
                    memory_total_bytes: if mem_res == NVML_SUCCESS {
                        Some(mem.total)
                    } else {
                        None
                    },
                    temperature_celsius: if temp_res == NVML_SUCCESS {
                        Some(temp as f32)
                    } else {
                        None
                    },
                    available: true,
                    source: Some("nvml".to_string()),
                })
            }
        }

        fn try_init() -> Option<NvmlLoaded> {
            unsafe {
                let mut lib = LoadLibraryA(c"nvml.dll".as_ptr());
                if lib.is_null() {
                    lib = LoadLibraryA(
                        c"C:\\Program Files\\NVIDIA Corporation\\NVSMI\\nvml.dll".as_ptr(),
                    );
                }
                if lib.is_null() {
                    return None;
                }

                let load_symbols = || -> Option<NvmlLoaded> {
                    let fn_init: NvmlInitFn =
                        get_proc(lib, c"nvmlInit_v2").or_else(|| get_proc(lib, c"nvmlInit"))?;
                    let fn_shutdown: NvmlShutdownFn = get_proc(lib, c"nvmlShutdown")?;
                    let fn_get_count: NvmlDeviceGetCountFn =
                        get_proc(lib, c"nvmlDeviceGetCount_v2")
                            .or_else(|| get_proc(lib, c"nvmlDeviceGetCount"))?;
                    let fn_get_handle: NvmlDeviceGetHandleByIndexFn =
                        get_proc(lib, c"nvmlDeviceGetHandleByIndex_v2")
                            .or_else(|| get_proc(lib, c"nvmlDeviceGetHandleByIndex"))?;
                    let fn_get_name: NvmlDeviceGetNameFn = get_proc(lib, c"nvmlDeviceGetName")?;
                    let fn_get_util: NvmlDeviceGetUtilizationRatesFn =
                        get_proc(lib, c"nvmlDeviceGetUtilizationRates")?;
                    let fn_get_mem: NvmlDeviceGetMemoryInfoFn =
                        get_proc(lib, c"nvmlDeviceGetMemoryInfo")?;
                    let fn_get_temp: NvmlDeviceGetTemperatureFn =
                        get_proc(lib, c"nvmlDeviceGetTemperature")?;

                    if fn_init() != NVML_SUCCESS {
                        return None;
                    }

                    let mut count: u32 = 0;
                    if fn_get_count(&mut count) != NVML_SUCCESS || count == 0 {
                        fn_shutdown();
                        return None;
                    }

                    let mut device: NvmlDevice = std::ptr::null_mut();
                    if fn_get_handle(0, &mut device) != NVML_SUCCESS || device.is_null() {
                        fn_shutdown();
                        return None;
                    }

                    let mut name_buf = [0i8; 96];
                    let name = if fn_get_name(device, name_buf.as_mut_ptr(), 95) == NVML_SUCCESS {
                        name_buf[95] = 0;
                        let cstr = CStr::from_ptr(name_buf.as_ptr());
                        cstr.to_string_lossy().into_owned()
                    } else {
                        "NVIDIA GPU".to_string()
                    };

                    Some(NvmlLoaded {
                        module: lib,
                        device,
                        name,
                        fn_shutdown,
                        fn_get_util,
                        fn_get_mem,
                        fn_get_temp,
                    })
                };

                let res = load_symbols();
                if res.is_none() {
                    FreeLibrary(lib);
                }
                res
            }
        }
    }
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub(crate) fn parse_nvidia_smi_line(line: &str) -> Option<GpuInfo> {
    let parts = line.split(',').map(|part| part.trim()).collect::<Vec<_>>();
    if parts.len() < 5 {
        return None;
    }

    let memory_used_mib = parts[2].parse::<u64>().ok();
    let memory_total_mib = parts[3].parse::<u64>().ok();

    Some(GpuInfo {
        name: Some(parts[0].to_string()),
        usage_percent: parts[1].parse::<f32>().ok(),
        memory_used_bytes: memory_used_mib.map(mib_to_bytes),
        memory_total_bytes: memory_total_mib.map(mib_to_bytes),
        temperature_celsius: parts[4].parse::<f32>().ok(),
        available: true,
        source: Some("nvidia-smi".to_string()),
    })
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn mib_to_bytes(value: u64) -> u64 {
    value.saturating_mul(1024 * 1024)
}

#[cfg(target_os = "macos")]
fn sample_macos_gpu_name() -> Option<String> {
    let output = std::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("Chipset Model:").map(str::trim))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    use super::parse_nvidia_smi_line;

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_nvidia_smi_output() {
        let info = parse_nvidia_smi_line("NVIDIA RTX 4070, 12, 1024, 8192, 45").unwrap();

        assert_eq!(info.name.as_deref(), Some("NVIDIA RTX 4070"));
        assert_eq!(info.usage_percent, Some(12.0));
        assert_eq!(info.memory_used_bytes, Some(1024 * 1024 * 1024));
        assert_eq!(info.temperature_celsius, Some(45.0));
    }

    #[test]
    fn gpu_sampler_handles_graceful_degradation() {
        let mut sampler = super::GpuSampler::new();
        let info = sampler.sample();
        if !info.available {
            assert!(info.name.is_none());
            assert!(info.usage_percent.is_none());
        }
    }
}
