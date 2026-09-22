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
    #[cfg(target_os = "windows")]
    dxgi_pdh_sampler: dxgi_pdh::DxgiPdhSampler,
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
            #[cfg(target_os = "windows")]
            dxgi_pdh_sampler: dxgi_pdh::DxgiPdhSampler::new(),
            #[cfg(target_os = "macos")]
            cached_name: None,
        }
    }

    pub fn sample(&mut self) -> GpuInfo {
        #[cfg(target_os = "windows")]
        {
            // Prefer discrete NVIDIA GPU if present and connected
            if let Some(info) = self.nvml_sampler.sample() {
                return info;
            }

            // Gracefully fall back to integrated GPU (Intel Arc / AMD / etc.) via DXGI + PDH
            if let Some(info) = self.dxgi_pdh_sampler.sample() {
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
mod dxgi_pdh {
    use std::ffi::{c_char, c_void, CStr};
    use std::time::{Duration, Instant};

    const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;
    const PDH_FMT_LARGE: u32 = 0x0000_0400;

    // GUID: {770aae78-f26f-4dba-a829-253c83d1b387}
    const IID_IDXGI_FACTORY1: [u8; 16] = [
        0x78, 0xae, 0x0a, 0x77, 0x6f, 0xf2, 0xba, 0x4d, 0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3,
        0x87,
    ];

    #[repr(C)]
    struct DxgiAdapterDesc1 {
        description: [u16; 128],
        vendor_id: u32,
        device_id: u32,
        sub_sys_id: u32,
        revision: u32,
        dedicated_video_memory: usize,
        dedicated_system_memory: usize,
        shared_system_memory: usize,
        adapter_luid_low: u32,
        adapter_luid_high: i32,
        flags: u32,
    }

    type CreateDxgiFactory1Fn =
        unsafe extern "system" fn(riid: *const [u8; 16], pp_factory: *mut *mut c_void) -> i32;
    type EnumAdapters1Fn = unsafe extern "system" fn(
        this: *mut c_void,
        adapter: u32,
        pp_adapter: *mut *mut c_void,
    ) -> i32;
    type GetDesc1Fn =
        unsafe extern "system" fn(this: *mut c_void, p_desc: *mut DxgiAdapterDesc1) -> i32;
    type ReleaseFn = unsafe extern "system" fn(this: *mut c_void) -> u32;

    #[repr(C)]
    #[derive(Default, Copy, Clone)]
    struct PdhFmtCounterValue {
        c_status: u32,
        _padding: u32,
        large_value: i64,
    }

    type PdhOpenQueryWFn = unsafe extern "system" fn(
        sz_data_source: *const u16,
        dw_user_data: usize,
        ph_query: *mut *mut c_void,
    ) -> i32;
    type PdhAddCounterWFn = unsafe extern "system" fn(
        h_query: *mut c_void,
        sz_path: *const u16,
        dw_user_data: usize,
        ph_counter: *mut *mut c_void,
    ) -> i32;
    type PdhCollectQueryDataFn = unsafe extern "system" fn(h_query: *mut c_void) -> i32;
    type PdhGetFormattedCounterValueFn = unsafe extern "system" fn(
        h_counter: *mut c_void,
        dw_format: u32,
        lpdw_type: *mut u32,
        p_value: *mut PdhFmtCounterValue,
    ) -> i32;
    type PdhCloseQueryFn = unsafe extern "system" fn(h_query: *mut c_void) -> i32;

    extern "system" {
        fn LoadLibraryA(lp_lib_file_name: *const c_char) -> *mut c_void;
        fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const c_char) -> *mut c_void;
        fn FreeLibrary(h_lib_module: *mut c_void) -> i32;
    }

    unsafe fn get_proc<T>(module: *mut c_void, name: &CStr) -> Option<T> {
        let proc = GetProcAddress(module, name.as_ptr());
        if proc.is_null() {
            None
        } else {
            Some(std::mem::transmute_copy(&proc))
        }
    }

    unsafe fn com_release(ptr: *mut c_void) -> u32 {
        if ptr.is_null() {
            return 0;
        }
        let vtable = *(ptr as *mut *const usize);
        let release_fn: ReleaseFn = std::mem::transmute(*vtable.add(2));
        release_fn(ptr)
    }

    #[derive(Debug, Clone)]
    struct AdapterInfo {
        name: String,
        dedicated_video_memory: u64,
        shared_system_memory: u64,
        luid_low: u32,
        luid_high: i32,
    }

    struct PdhSession {
        module: *mut c_void,
        h_query: *mut c_void,
        h_dedicated: *mut c_void,
        h_shared: *mut c_void,
        fn_collect: PdhCollectQueryDataFn,
        fn_get_value: PdhGetFormattedCounterValueFn,
        fn_close: PdhCloseQueryFn,
    }

    unsafe impl Send for PdhSession {}

    impl Drop for PdhSession {
        fn drop(&mut self) {
            unsafe {
                if !self.h_query.is_null() {
                    (self.fn_close)(self.h_query);
                }
                if !self.module.is_null() {
                    FreeLibrary(self.module);
                }
            }
        }
    }

    pub struct DxgiPdhSampler {
        adapter: Option<AdapterInfo>,
        pdh_session: Option<PdhSession>,
        last_check: Option<Instant>,
    }

    unsafe impl Send for DxgiPdhSampler {}

    impl std::fmt::Debug for DxgiPdhSampler {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("DxgiPdhSampler")
                .field("has_adapter", &self.adapter.is_some())
                .field("has_pdh", &self.pdh_session.is_some())
                .finish()
        }
    }

    const RECHECK_INTERVAL: Duration = Duration::from_secs(30);

    impl DxgiPdhSampler {
        pub fn new() -> Self {
            Self {
                adapter: None,
                pdh_session: None,
                last_check: None,
            }
        }

        pub fn sample(&mut self) -> Option<super::GpuInfo> {
            if self.adapter.is_none() {
                if let Some(last) = self.last_check {
                    if last.elapsed() < RECHECK_INTERVAL {
                        return None;
                    }
                }
                self.last_check = Some(Instant::now());
                self.adapter = Self::find_hardware_adapter();
                if let Some(ref adapter) = self.adapter {
                    self.pdh_session = Self::init_pdh(adapter);
                }
            }

            let adapter = self.adapter.as_ref()?;
            let total_memory = adapter
                .dedicated_video_memory
                .saturating_add(adapter.shared_system_memory);

            let (used_memory, usage_percent) = if let Some(ref mut pdh) = self.pdh_session {
                unsafe {
                    let collect_res = (pdh.fn_collect)(pdh.h_query);
                    if collect_res == 0 {
                        let mut val_ded = PdhFmtCounterValue::default();
                        let mut val_shared = PdhFmtCounterValue::default();
                        let mut val_type = 0u32;

                        let mut used: u64 = 0;
                        if !pdh.h_dedicated.is_null()
                            && (pdh.fn_get_value)(
                                pdh.h_dedicated,
                                PDH_FMT_LARGE,
                                &mut val_type,
                                &mut val_ded,
                            ) == 0
                            && val_ded.c_status == 0
                        {
                            used = used.saturating_add(val_ded.large_value.max(0) as u64);
                        }
                        if !pdh.h_shared.is_null()
                            && (pdh.fn_get_value)(
                                pdh.h_shared,
                                PDH_FMT_LARGE,
                                &mut val_type,
                                &mut val_shared,
                            ) == 0
                            && val_shared.c_status == 0
                        {
                            used = used.saturating_add(val_shared.large_value.max(0) as u64);
                        }

                        let pct = if total_memory > 0 {
                            Some(((used as f64 / total_memory as f64) * 100.0) as f32)
                        } else {
                            None
                        };

                        (Some(used), pct)
                    } else {
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };

            Some(super::GpuInfo {
                name: Some(adapter.name.clone()),
                usage_percent,
                memory_used_bytes: used_memory,
                memory_total_bytes: if total_memory > 0 {
                    Some(total_memory)
                } else {
                    None
                },
                temperature_celsius: None,
                available: true,
                source: Some("dxgi_pdh".to_string()),
            })
        }

        fn find_hardware_adapter() -> Option<AdapterInfo> {
            unsafe {
                let dxgi = LoadLibraryA(c"dxgi.dll".as_ptr());
                if dxgi.is_null() {
                    return None;
                }

                let create_factory: Option<CreateDxgiFactory1Fn> =
                    get_proc(dxgi, c"CreateDXGIFactory1");
                let Some(create_factory) = create_factory else {
                    FreeLibrary(dxgi);
                    return None;
                };

                let mut factory: *mut c_void = std::ptr::null_mut();
                if create_factory(&IID_IDXGI_FACTORY1, &mut factory) != 0 || factory.is_null() {
                    FreeLibrary(dxgi);
                    return None;
                }

                let vtbl_f = *(factory as *mut *const usize);
                let enum_adapters: EnumAdapters1Fn = std::mem::transmute(*vtbl_f.add(12));

                let mut candidates = Vec::new();
                let mut idx = 0u32;
                loop {
                    let mut adapter: *mut c_void = std::ptr::null_mut();
                    if enum_adapters(factory, idx, &mut adapter) != 0 || adapter.is_null() {
                        break;
                    }

                    let vtbl_a = *(adapter as *mut *const usize);
                    let get_desc1: GetDesc1Fn = std::mem::transmute(*vtbl_a.add(10));

                    let mut desc = std::mem::zeroed::<DxgiAdapterDesc1>();
                    if get_desc1(adapter, &mut desc) == 0
                        && (desc.flags & DXGI_ADAPTER_FLAG_SOFTWARE) == 0
                    {
                        let len = desc
                            .description
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(desc.description.len());
                        let name = String::from_utf16_lossy(&desc.description[..len])
                            .trim()
                            .to_string();

                        if !name.is_empty() {
                            candidates.push(AdapterInfo {
                                name,
                                dedicated_video_memory: desc.dedicated_video_memory as u64,
                                shared_system_memory: desc.shared_system_memory as u64,
                                luid_low: desc.adapter_luid_low,
                                luid_high: desc.adapter_luid_high,
                            });
                        }
                    }

                    com_release(adapter);
                    idx += 1;
                }

                com_release(factory);
                FreeLibrary(dxgi);

                // Prefer non-NVIDIA adapter (e.g. Intel Arc / AMD) since NVIDIA is handled by NVML
                candidates
                    .into_iter()
                    .min_by_key(|a| if a.name.contains("NVIDIA") { 1 } else { 0 })
            }
        }

        fn init_pdh(adapter: &AdapterInfo) -> Option<PdhSession> {
            unsafe {
                let pdh_mod = LoadLibraryA(c"pdh.dll".as_ptr());
                if pdh_mod.is_null() {
                    return None;
                }

                let fn_open: Option<PdhOpenQueryWFn> = get_proc(pdh_mod, c"PdhOpenQueryW");
                let fn_add: Option<PdhAddCounterWFn> = get_proc(pdh_mod, c"PdhAddEnglishCounterW")
                    .or_else(|| get_proc(pdh_mod, c"PdhAddCounterW"));
                let fn_collect: Option<PdhCollectQueryDataFn> =
                    get_proc(pdh_mod, c"PdhCollectQueryData");
                let fn_get_value: Option<PdhGetFormattedCounterValueFn> =
                    get_proc(pdh_mod, c"PdhGetFormattedCounterValue");
                let fn_close: Option<PdhCloseQueryFn> = get_proc(pdh_mod, c"PdhCloseQuery");

                let (
                    Some(fn_open),
                    Some(fn_add),
                    Some(fn_collect),
                    Some(fn_get_value),
                    Some(fn_close),
                ) = (fn_open, fn_add, fn_collect, fn_get_value, fn_close)
                else {
                    FreeLibrary(pdh_mod);
                    return None;
                };

                let mut h_query: *mut c_void = std::ptr::null_mut();
                if fn_open(std::ptr::null(), 0, &mut h_query) != 0 || h_query.is_null() {
                    FreeLibrary(pdh_mod);
                    return None;
                }

                let to_wide =
                    |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };

                let path_dedicated = to_wide(&format!(
                    "\\GPU Adapter Memory(luid_0x{:08x}_0x{:08x}_phys_0)\\Dedicated Usage",
                    adapter.luid_high, adapter.luid_low
                ));
                let path_shared = to_wide(&format!(
                    "\\GPU Adapter Memory(luid_0x{:08x}_0x{:08x}_phys_0)\\Shared Usage",
                    adapter.luid_high, adapter.luid_low
                ));

                let mut h_dedicated: *mut c_void = std::ptr::null_mut();
                let _ = fn_add(h_query, path_dedicated.as_ptr(), 0, &mut h_dedicated);

                let mut h_shared: *mut c_void = std::ptr::null_mut();
                let _ = fn_add(h_query, path_shared.as_ptr(), 0, &mut h_shared);

                // Collect baseline data
                let _ = fn_collect(h_query);

                Some(PdhSession {
                    module: pdh_mod,
                    h_query,
                    h_dedicated,
                    h_shared,
                    fn_collect,
                    fn_get_value,
                    fn_close,
                })
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
