//! 2-Pyramid Win11 右键菜单 COM 服务端
//!
//! 实现 IExplorerCommand + IEnumExplorerCommand，在 Win11 精简版右键菜单中
//! 显示 "转换版本至" 级联菜单。

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::os::windows::process::CommandExt;

use windows_core::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::Foundation::*;

// ── 全局锁计数 ──
static LOCK_COUNT: AtomicU32 = AtomicU32::new(0);
static EXE_PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

// ── CLSID ──
const CLSID_HURRICANE: GUID = GUID::from_values(
    0xB8A1F3D2, 0x7C6E, 0x4A9B, [0x8F, 0x2D, 0x1E, 0x3C, 0x5A, 0x7B, 0x9D, 0x0F]
);

// ── 版本数据（与 registry.rs 保持同步） ──
const VERSIONS: &[(&str, &str)] = &[
    ("Java 1.6-1.8", "1"),
    ("Java 1.9-1.10", "2"),
    ("Java 1.11-1.12", "3"),
    ("Java 1.13-1.14", "4"),
    ("Java 1.15-1.16.1", "5"),
    ("Java 1.16.2-1.16.5", "6"),
    ("Java 1.17", "7"),
    ("Java 1.18", "8"),
    ("Java 1.19-1.19.2", "9"),
    ("Java 1.19.3", "12"),
    ("Java 1.19.4", "13"),
    ("Java 1.20-1.20.1", "15"),
    ("Java 1.20.2", "18"),
    ("Java 1.20.3-1.20.4", "22"),
    ("Java 1.20.5-1.20.6", "32"),
    ("Java 1.21-1.21.1", "34"),
    ("Java 1.21.2-1.21.3", "42"),
    ("Java 1.21.4", "46"),
    ("Java 1.21.5", "55"),
    ("Java 1.21.6", "63"),
    ("Java 1.21.7-1.21.8", "64"),
    ("Java 1.21.9-1.21.10", "69"),
    ("Java 1.21.11", "75"),
    ("Java 26.1-26.1.2", "84"),
    ("Bedrock Latest", "1000"),
];

// ── 辅助 ──

fn alloc_pwstr(s: &str) -> PWSTR {
    let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = wide.len() * 2;
    let ptr = unsafe {
        let raw = CoTaskMemAlloc(bytes);
        if raw.is_null() {
            std::ptr::null_mut()
        } else {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), raw as *mut u16, wide.len());
            raw
        }
    };
    PWSTR::from_raw(ptr as *mut u16)
}

fn get_exe_path() -> String {
    EXE_PATH.get()
        .and_then(|p| std::path::Path::new(p).parent())
        .map(|d| format!("{}\\2-pyramid.exe", d.to_string_lossy().replace('/', "\\")))
        .unwrap_or_default()
}

fn get_file_path(items: Ref<'_, IShellItemArray>) -> Option<String> {
    if items.is_null() {
        return None;
    }
    let isa: &IShellItemArray = items.ok().ok()?;
    let count = unsafe { isa.GetCount() }.ok()?;
    if count == 0 {
        return None;
    }
    let item = unsafe { isa.GetItemAt(0) }.ok()?;
    let name = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.ok()?;
    unsafe {
        let ptr = name.0;
        if ptr.is_null() {
            return None;
        }
        let len = (0..).take_while(|&i| *ptr.add(i) != 0).count();
        Some(String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len)))
    }
}

fn launch_conversion(file_path: &str, format_id: &str) {
    let exe = get_exe_path();
    if exe.is_empty() {
        return;
    }
    let _ = std::process::Command::new(&exe)
        .args(["--convert", file_path, "--format", format_id])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn();
}

fn err_notimpl() -> Error { Error::from_hresult(E_NOTIMPL) }

// ═══════════════════════════════════════════
// 子命令 IExplorerCommand
// ═══════════════════════════════════════════

#[implement(IExplorerCommand)]
struct SubCommand {
    label: String,
    format_id: String,
}

impl SubCommand {
    fn new(label: &str, format_id: &str) -> Self {
        Self { label: label.into(), format_id: format_id.into() }
    }
}

impl IExplorerCommand_Impl for SubCommand_Impl {
    fn GetTitle(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        Ok(alloc_pwstr(&self.label))
    }

    fn GetIcon(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        Ok(PWSTR::null())
    }

    fn GetToolTip(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        Ok(PWSTR::null())
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Err(err_notimpl())
    }

    fn GetState(&self, _items: Ref<'_, IShellItemArray>, _ok_to_be_slow: BOOL) -> Result<u32> {
        Ok(ECS_ENABLED.0 as u32)
    }

    fn Invoke(&self, items: Ref<'_, IShellItemArray>, _bind_ctx: Ref<'_, IBindCtx>) -> Result<()> {
        if let Some(path) = get_file_path(items) {
            launch_conversion(&path, &self.format_id);
        }
        Ok(())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(0)
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        Err(err_notimpl())
    }
}

// ═══════════════════════════════════════════
// IEnumExplorerCommand
// ═══════════════════════════════════════════

#[implement(IEnumExplorerCommand)]
struct EnumExplorerCommand {
    items: Vec<IExplorerCommand>,
    index: RefCell<usize>,
}

impl IEnumExplorerCommand_Impl for EnumExplorerCommand_Impl {
    fn Next(&self, celt: u32, rgelt: *mut Option<IExplorerCommand>, pceltfetched: *mut u32) -> HRESULT {
        let mut idx = self.index.borrow_mut();
        let mut fetched: u32 = 0;

        for i in 0..celt as usize {
            if *idx >= self.items.len() {
                break;
            }
            unsafe {
                *rgelt.add(i) = Some(self.items[*idx].clone());
            }
            *idx += 1;
            fetched += 1;
        }

        if !pceltfetched.is_null() {
            unsafe { *pceltfetched = fetched; }
        }

        if fetched == celt { S_OK } else { S_FALSE }
    }

    fn Skip(&self, celt: u32) -> Result<()> {
        let mut idx = self.index.borrow_mut();
        let remaining = self.items.len().saturating_sub(*idx);
        if (celt as usize) > remaining {
            *idx = self.items.len();
            Err(err_notimpl())
        } else {
            *idx += celt as usize;
            Ok(())
        }
    }

    fn Reset(&self) -> Result<()> {
        *self.index.borrow_mut() = 0;
        Ok(())
    }

    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        let cloned = EnumExplorerCommand {
            items: self.items.clone(),
            index: RefCell::new(*self.index.borrow()),
        };
        Ok(cloned.into())
    }
}

// ═══════════════════════════════════════════
// 父级 IExplorerCommand
// ═══════════════════════════════════════════

#[implement(IExplorerCommand)]
struct ParentCommand;

impl IExplorerCommand_Impl for ParentCommand_Impl {
    fn GetTitle(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        Ok(alloc_pwstr("转换版本至"))
    }

    fn GetIcon(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        let exe = get_exe_path();
        if exe.is_empty() {
            return Ok(PWSTR::null());
        }
        Ok(alloc_pwstr(&format!("{},0", exe)))
    }

    fn GetToolTip(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        Ok(alloc_pwstr("将此资源包转换为指定 Minecraft 版本"))
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Err(err_notimpl())
    }

    fn GetState(&self, _items: Ref<'_, IShellItemArray>, _ok_to_be_slow: BOOL) -> Result<u32> {
        Ok(ECS_ENABLED.0 as u32)
    }

    fn Invoke(&self, _items: Ref<'_, IShellItemArray>, _bind_ctx: Ref<'_, IBindCtx>) -> Result<()> {
        Ok(())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(ECF_HASSUBCOMMANDS.0 as u32)
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        let sub_items: Vec<IExplorerCommand> = VERSIONS.iter()
            .map(|(label, fmt)| {
                let cmd: IExplorerCommand = SubCommand::new(label, fmt).into();
                cmd
            })
            .collect();
        let enumerator: IEnumExplorerCommand = EnumExplorerCommand {
            items: sub_items,
            index: RefCell::new(0),
        }.into();
        Ok(enumerator)
    }
}

// ═══════════════════════════════════════════
// IClassFactory
// ═══════════════════════════════════════════

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppv: *mut *mut std::ffi::c_void,
    ) -> Result<()> {
        if !outer.is_null() {
            return Err(Error::from_hresult(CLASS_E_NOAGGREGATION));
        }
        let parent: IExplorerCommand = ParentCommand.into();
        unsafe { parent.query(riid, ppv).ok() }
    }

    fn LockServer(&self, lock: BOOL) -> Result<()> {
        if lock.as_bool() {
            LOCK_COUNT.fetch_add(1, Ordering::SeqCst);
        } else {
            LOCK_COUNT.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════
// DLL 导出
// ═══════════════════════════════════════════

fn ensure_exe_path() {
    let _ = EXE_PATH.get_or_init(|| {
        std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT {
    ensure_exe_path();

    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_INVALIDARG;
    }
    unsafe {
        if *rclsid != CLSID_HURRICANE {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        let factory: IClassFactory = ClassFactory.into();
        factory.query(riid, ppv)
    }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if LOCK_COUNT.load(Ordering::SeqCst) == 0 { S_OK } else { S_FALSE }
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT { S_OK }

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT { S_OK }
